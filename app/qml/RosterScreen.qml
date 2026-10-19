import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.rosterJson()); }
    property string filter: "All"
    property string sortKey: "ovr"
    property bool sortDesc: true
    readonly property var groups: ["All", "QB", "RB", "WR", "TE", "OL", "DL", "LB", "DB", "K", "P"]
    readonly property var posMap: ({ "RB": ["RB", "FB"], "OL": ["OT", "OG", "C"], "DL": ["DE", "DT"], "LB": ["MLB", "OLB"], "DB": ["CB", "S"] })
    readonly property bool wide: width >= 760

    readonly property var rows: {
        var rs = (vm.rows || []).filter(function(p) {
            if (filter === "All") return true;
            var ps = posMap[filter] || [filter];
            return ps.indexOf(p.pos) >= 0;
        });
        var k = sortKey, d = sortDesc ? -1 : 1;
        rs.sort(function(a, b) {
            var x = a[k], y = b[k];
            if (x === null || x === undefined) x = -1;
            if (y === null || y === undefined) y = -1;
            if (typeof x === "string") return d * x.localeCompare(y);
            return d * (x - y) || a.name.localeCompare(b.name);
        });
        return rs;
    }
    function sortBy(k) { if (sortKey === k) sortDesc = !sortDesc; else { sortKey = k; sortDesc = k !== "name" && k !== "pos"; } }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.gutter
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            ScreenTitle { text: "Roster"; Layout.fillWidth: true }
            Text { text: (root.vm.active || 0) + " / " + (root.vm.activeMax || 46) + " active  ·  " + (root.vm.total || 0) + " total  ·  payroll " + (root.vm.payrollText || ""); color: Theme.inkSubtle; font.pixelSize: 13; visible: root.width >= 520 }
        }
        Text {
            visible: (root.vm.gaps || []).length > 0
            text: "Active list gaps: " + (root.vm.gaps || []).join("  ·  ")
            color: Theme.warnFg; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true
        }
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater { model: root.groups; Chip { text: modelData; selected: root.filter === modelData; onClicked: root.filter = modelData } }
            Check { checked: root.vm.autoSet; onToggled: Store.game.setAutoRoster(checked) }
            Text { text: "Auto roster"; color: Theme.inkMuted; font.pixelSize: 13; height: 32; verticalAlignment: Text.AlignVCenter }
            PrimaryButton { text: "Auto-set"; ghost: true; implicitHeight: 32; font.pixelSize: 13; onClicked: Store.game.autoSetActive() }
        }

        // header row
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 32
            color: Theme.surface
            radius: Theme.radiusChip
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 10; anchors.rightMargin: 10
                spacing: 8
                Text { text: "Act"; color: Theme.inkSubtle; font.pixelSize: 12; font.bold: true; Layout.preferredWidth: 34 }
                Repeater {
                    model: [ { k: "name", t: "Player", w: -1 }, { k: "pos", t: "Pos", w: 44 }, { k: "ovr", t: "OVR", w: 46 }, { k: "age", t: "Age", w: 40 }, { k: "ppg", t: "Pay/gm", w: root.wide ? 70 : 0 }, { k: "years", t: "Yrs", w: root.wide ? 36 : 0 }, { k: "status", t: "Status", w: root.wide ? 80 : 0 } ]
                    Text {
                        visible: modelData.w !== 0
                        text: modelData.t + (root.sortKey === modelData.k ? (root.sortDesc ? " ▼" : " ▲") : "")
                        color: root.sortKey === modelData.k ? Theme.primary : Theme.inkSubtle
                        font.pixelSize: 12; font.bold: true
                        Layout.fillWidth: modelData.w < 0
                        Layout.preferredWidth: modelData.w < 0 ? 0 : modelData.w
                        TapHandler { onTapped: root.sortBy(modelData.k) }
                    }
                }
            }
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.rows
            spacing: 2
            property real savedY: -1   // the model is rebuilt after every change, which would jump back to the top
            onModelChanged: if (savedY >= 0) Qt.callLater(function() { list.contentY = list.savedY; list.savedY = -1; })
            ScrollBar.vertical: ScrollBar {}
            delegate: Rectangle {
                width: ListView.view.width - 8
                height: 40
                radius: Theme.radiusChip
                color: hov.hovered ? Theme.surfaceAlt : (modelData.active ? Theme.surface : "transparent")
                opacity: modelData.active ? 1 : 0.62
                HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: Store.openPlayer(modelData.id) }
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 10; anchors.rightMargin: 10
                    spacing: 8
                    Check {
                        checked: modelData.active
                        Layout.preferredWidth: 34
                        padding: 0
                        onToggled: { list.savedY = list.contentY; if (!Store.game.setActive(modelData.id, checked)) { list.savedY = -1; checked = Qt.binding(function() { return modelData.active; }); } }
                    }
                    Text { text: modelData.name + (modelData.local ? "  ★" : "") ; color: modelData.out ? Theme.badFg : Theme.ink; font.pixelSize: 14; elide: Text.ElideRight; Layout.fillWidth: true }
                    Text { text: modelData.pos; color: Theme.inkMuted; font.pixelSize: 13; Layout.preferredWidth: 44 }
                    Text { text: modelData.ovr; color: Theme.ovrColor(modelData.ovr); font.pixelSize: 14; font.bold: true; Layout.preferredWidth: 46 }
                    Text { text: modelData.age; color: Theme.inkMuted; font.pixelSize: 13; Layout.preferredWidth: 40 }
                    Text { visible: root.wide; text: modelData.ppg !== null ? Theme.money(modelData.ppg) : "–"; color: Theme.inkMuted; font.pixelSize: 13; Layout.preferredWidth: 70 }
                    Text { visible: root.wide; text: modelData.years !== null ? modelData.years : "–"; color: Theme.inkMuted; font.pixelSize: 13; Layout.preferredWidth: 36 }
                    Text { visible: root.wide; text: modelData.status; color: modelData.out ? Theme.badFg : (modelData.status === "Expiring" ? Theme.warnFg : Theme.inkFaint); font.pixelSize: 12; Layout.preferredWidth: 80 }
                }
            }
        }
        Text { text: "★ local player (fans love them)"; color: Theme.inkFaint; font.pixelSize: 11 }
    }
}
