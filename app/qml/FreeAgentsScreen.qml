import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    property string pos: "All"
    property string sort: "OVR"
    property string notice: ""
    property bool showTitle: true
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.freeAgentsJson()); }
    readonly property var groupMap: ({ "RB": ["RB", "FB"], "OL": ["OT", "OG", "C"], "DL": ["DE", "DT"], "LB": ["MLB", "OLB"], "DB": ["CB", "S"] })
    function inGroup(label, p) { var ps = groupMap[label] || [label]; return ps.indexOf(p.pos) >= 0; }

    readonly property var rows: {
        var rs = (vm.rows || []).filter(function(p) {
            if (pos === "All") return true;
            if (pos === "Needs") { for (var i = 0; i < vm.needs.length; i++) if (inGroup(vm.needs[i], p)) return true; return false; }
            return inGroup(pos, p);
        });
        if (sort === "Local only") rs = rs.filter(function(p) { return p.local; });
        rs.sort(function(a, b) { return sort === "Age" ? a.age - b.age : (sort === "Cheapest" ? a.ask - b.ask : b.ovr - a.ovr); });
        return rs.slice(0, 60);
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: root.showTitle ? Theme.gutter : 0
        spacing: 10
        RowLayout {
            Layout.fillWidth: true
            ScreenTitle { visible: root.showTitle; text: "Free agents"; Layout.fillWidth: true }
            Item { visible: !root.showTitle; Layout.fillWidth: true }
            Text { text: (root.vm.rosterCount || 0) + " on roster  ·  " + (root.vm.active || 0) + " / " + (root.vm.activeMax || 46) + " active  ·  cash " + (root.vm.cashText || ""); color: Theme.inkSubtle; font.pixelSize: 13; visible: root.width >= 560 }
        }
        Card {
            visible: (root.vm.needs || []).length > 0
            fill: Theme.warnBg
            Text { text: "Short-handed at " + (root.vm.needs || []).join(", ") + ". Others will play out of position until you sign someone."; color: Theme.warnFg; font.pixelSize: 13; font.bold: true; wrapMode: Text.WordWrap; Layout.fillWidth: true }
        }
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater { model: ["All"].concat((root.vm.needs || []).length > 0 ? ["Needs"] : []).concat(root.vm.groups || []); Chip { text: modelData; selected: root.pos === modelData; onClicked: root.pos = modelData } }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 6
            Text { text: "Sort"; color: Theme.inkSubtle; font.pixelSize: 12 }
            Repeater { model: ["OVR", "Age", "Cheapest", "Local only"]; Chip { text: modelData; selected: root.sort === modelData; implicitHeight: 28; font.pixelSize: 12; onClicked: root.sort = modelData } }
            Item { Layout.fillWidth: true }
            PrimaryButton {
                implicitHeight: 32; font.pixelSize: 13; ghost: true
                enabled: !!root.vm.canTryout
                text: root.vm.tryoutHeld ? "Tryouts held this week" : (root.pos === "All" || root.pos === "Needs" ? "Hold open tryouts" : "Tryouts: " + root.pos) + " (" + (root.vm.tryoutCostText || "") + ")"
                onClicked: { var n = Store.game.holdTryouts(root.pos === "All" || root.pos === "Needs" ? "" : root.pos); root.notice = n + " hopefuls showed up."; }
            }
        }
        Text { visible: root.notice !== ""; text: root.notice; color: Theme.goodFg; font.pixelSize: 12 }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 6
            model: root.rows
            ScrollBar.vertical: ScrollBar {}
            delegate: Rectangle {
                width: ListView.view.width - 8
                height: 70
                radius: Theme.radiusChip + 2
                color: hov.hovered ? Theme.surfaceAlt : Theme.surface
                border.color: Theme.divider
                HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: Store.openPlayer(modelData.id) }
                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 12
                    Rectangle { width: 42; height: 42; radius: 10; color: Theme.surfaceAlt; Text { anchors.centerIn: parent; text: modelData.ovr; color: Theme.ovrColor(modelData.ovr); font.bold: true; font.pixelSize: 18 } }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        RowLayout {
                            spacing: 6
                            Text { text: modelData.name; color: Theme.ink; font.pixelSize: 15; font.bold: true; elide: Text.ElideRight; Layout.maximumWidth: 240 }
                            Pill { visible: modelData.local; text: "LOCAL"; fg: Theme.primary; bg: Theme.primaryTint; implicitHeight: 18; strong: true }
                        }
                        Text { text: modelData.pos + " · " + modelData.age + " · " + modelData.size + " · " + modelData.hometown; color: Theme.inkSubtle; font.pixelSize: 12; elide: Text.ElideRight; Layout.fillWidth: true }
                        Row {
                            spacing: 6
                            Repeater { model: modelData.keys; Pill { text: modelData.code + " " + modelData.v; implicitHeight: 18; fg: modelData.v >= 65 ? Theme.goodFg : (modelData.v >= 45 ? Theme.warnFg : Theme.badFg); bg: modelData.v >= 65 ? Theme.goodBg : (modelData.v >= 45 ? Theme.warnBg : Theme.badBg) } }
                        }
                    }
                    ColumnLayout {
                        spacing: 4
                        Text { text: modelData.askText + "/g"; color: Theme.primary; font.bold: true; font.pixelSize: 14; Layout.alignment: Qt.AlignRight }
                        PrimaryButton { text: "Offer"; implicitHeight: 30; font.pixelSize: 13; onClicked: Store.openNegotiation(modelData.id) }
                    }
                }
            }
        }
    }
}
