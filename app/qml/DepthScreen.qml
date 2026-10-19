import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    property string unit: "Offense"
    property string offPkg: ""
    property string defPkg: ""
    property var addGroup: null
    property var candidates: []
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.depthJson()); }
    readonly property var unitData: { var us = vm.units || []; for (var i = 0; i < us.length; i++) if (us[i].name === unit) return us[i]; return ({ groups: [] }); }
    readonly property string curPkg: unit === "Offense" ? (offPkg || vm.offDefault) : (unit === "Defense" ? (defPkg || vm.defDefault) : "")
    readonly property var pkgList: unit === "Offense" ? (vm.offensePackages || []) : (unit === "Defense" ? (vm.defensePackages || []) : [])
    readonly property var pkgSlots: { for (var i = 0; i < pkgList.length; i++) if (pkgList[i].name === curPkg) return pkgList[i].slots; return ({}); }
    readonly property bool fits: unit === "Offense" ? curPkg === vm.offDefault : curPkg === vm.defDefault
    readonly property var unitColor: ({ "Offense": "#3b6b4a", "Defense": "#3d59a1", "Special": "#9a6a2e" })

    function idsOf(g) { return g.rows.map(function(r) { return r.id; }); }
    function move(g, from, to) {
        var ids = idsOf(g);
        ids.splice(to, 0, ids.splice(from, 1)[0]);
        Store.game.setDepth(g.key, JSON.stringify(ids));
    }
    function remove(g, id) { Store.game.setDepth(g.key, JSON.stringify(idsOf(g).filter(function(x) { return x !== id; }))); }
    function slotsFor(g) { return unit === "Special" ? 1 : (pkgSlots[g.key] || 0); }
    function openAdd(g) { addGroup = g; candidates = JSON.parse(Store.game.depthCandidatesJson(g.key)); addDlg.open(); }
    function add(id) {
        var ids = idsOf(addGroup).filter(function(x) { return x !== id; });
        ids.splice(Math.min(slotsFor(addGroup), ids.length), 0, id);   // first backup slot is where a new man earns a look
        Store.game.setDepth(addGroup.key, JSON.stringify(ids));
        addDlg.close();
    }

    component GroupCard: Rectangle {
        id: gcard
        property var g
        property int slots: 1
        Layout.fillWidth: true
        implicitHeight: col.implicitHeight
        radius: Theme.radiusCard
        color: Theme.surface
        border.color: Theme.divider
        clip: true
        ColumnLayout {
            id: col
            width: parent.width
            spacing: 0
            Rectangle {
                Layout.fillWidth: true; Layout.preferredHeight: 42
                color: root.unitColor[root.unit]
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 12; anchors.rightMargin: 6; spacing: 8
                    Text { text: gcard.g.title; color: "#ffffff"; font.bold: true; font.pixelSize: 14; Layout.fillWidth: true }
                    Pill { text: gcard.g.custom ? "Custom" : "Auto"; bg: "#33ffffff"; fg: "#ffffff"; implicitHeight: 22 }
                    PrimaryButton { visible: gcard.g.custom; text: "Reset"; ghost: true; implicitHeight: 28; font.pixelSize: 12; onClicked: Store.game.autoSort(gcard.g.key) }
                    PrimaryButton { text: "+"; ghost: true; implicitHeight: 28; implicitWidth: 36; leftPadding: 4; rightPadding: 4; onClicked: root.openAdd(gcard.g) }
                }
            }
            Text { visible: gcard.g.rows.length === 0; text: "Nobody active here."; color: Theme.inkMuted; padding: 12 }
            Repeater {
                model: gcard.g.rows
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    Text { visible: index === gcard.slots && gcard.slots > 0; text: "BACKUPS"; color: Theme.inkSubtle; font.pixelSize: 11; font.bold: true; font.letterSpacing: 1; leftPadding: 14; topPadding: 8; bottomPadding: 2 }
                    Rectangle {
                        Layout.fillWidth: true; implicitHeight: 52
                        color: modelData.out !== null && modelData.out !== undefined ? Theme.badBg : (index < gcard.slots ? Theme.primaryTint : "transparent")
                        HoverHandler { id: hv; cursorShape: Qt.PointingHandCursor }
                        TapHandler { onTapped: Store.openPlayer(modelData.id) }
                        RowLayout {
                            anchors.fill: parent; anchors.leftMargin: 12; anchors.rightMargin: 6; spacing: 8
                            Text { text: index + 1; color: Theme.inkFaint; font.bold: true; font.pixelSize: 13; Layout.preferredWidth: 14 }
                            Rectangle { width: 34; height: 34; radius: 8; color: Theme.surfaceAlt; Text { anchors.centerIn: parent; text: modelData.rate; color: Theme.ovrColor(modelData.rate); font.bold: true; font.pixelSize: 15 } }
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 1
                                Text { text: "#" + (modelData.number !== null ? modelData.number : "–") + " " + modelData.name; color: Theme.ink; font.bold: true; font.pixelSize: 14; elide: Text.ElideRight; Layout.fillWidth: true }
                                Row { spacing: 8
                                    Text { text: modelData.pos + (modelData.offPos ? " (out of position)" : ""); color: modelData.offPos ? Theme.warnFg : Theme.inkSubtle; font.pixelSize: 11 }
                                    Repeater { model: modelData.attrs; Text { text: modelData.code + " " + modelData.v; color: Theme.inkMuted; font.pixelSize: 11 } }
                                    Text { visible: modelData.out !== null && modelData.out !== undefined; text: "OUT " + modelData.out + " gm"; color: Theme.badFg; font.pixelSize: 11; font.bold: true }
                                }
                            }
                            PrimaryButton { text: "▲"; ghost: true; implicitHeight: 28; implicitWidth: 32; leftPadding: 4; rightPadding: 4; enabled: index > 0; onClicked: root.move(gcard.g, index, index - 1) }
                            PrimaryButton { text: "▼"; ghost: true; implicitHeight: 28; implicitWidth: 32; leftPadding: 4; rightPadding: 4; enabled: index < gcard.g.rows.length - 1; onClicked: root.move(gcard.g, index, index + 1) }
                            PrimaryButton { visible: modelData.offPos; text: "✕"; ghost: true; implicitHeight: 28; implicitWidth: 32; leftPadding: 4; rightPadding: 4; onClicked: root.remove(gcard.g, modelData.id) }
                        }
                    }
                }
            }
            Item { Layout.preferredHeight: 4 }
        }
    }

    ScrollView {
        id: sv
        anchors.fill: parent
        contentWidth: availableWidth
        clip: true
        ColumnLayout {
            width: sv.availableWidth - 2 * Theme.gutter
            x: Theme.gutter; y: Theme.gutter
            spacing: 12
            ScreenTitle { text: "Depth chart" }
            Card {
                fill: Theme.primaryTint
                Flow { Layout.fillWidth: true; spacing: 8
                    Pill { text: root.vm.customCount ? root.vm.customCount + " custom" : "All auto"; strong: true }
                    Pill { visible: root.pkgList.length > 0; text: root.fits ? "Matches strategy" : "Planning view"; strong: true }
                    Check { checked: root.vm.autoDepth; onToggled: Store.game.setAutoDepth(checked) }
                    Text { text: "Auto depth chart"; color: Theme.ink; font.pixelSize: 13; height: 30; verticalAlignment: Text.AlignVCenter }
                    PrimaryButton { text: "Auto sort all"; ghost: true; implicitHeight: 30; font.pixelSize: 13; onClicked: Store.game.autoSortAll() }
                }
                Text { text: "Slot 1 starts; the number of starters depends on the formation. Use the arrows to reorder, + to list someone, Reset to return a position to automatic."; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            }
            RowLayout { spacing: 6
                Repeater { model: ["Offense", "Defense", "Special"]; Chip { text: modelData; selected: root.unit === modelData; onClicked: root.unit = modelData } }
            }
            Card {
                visible: root.pkgList.length > 0
                title: (root.unit === "Offense" ? "Offensive" : "Defensive") + " formation"
                Flow { Layout.fillWidth: true; spacing: 6
                    Repeater { model: root.pkgList; Chip { text: modelData.name + (modelData.name === (root.unit === "Offense" ? root.vm.offDefault : root.vm.defDefault) ? " ★" : ""); selected: modelData.name === root.curPkg; onClicked: { if (root.unit === "Offense") root.offPkg = modelData.name; else root.defPkg = modelData.name; } } }
                }
                Text { text: "★ is what your strategy runs. Starter counts below follow the formation you pick."; color: Theme.inkSubtle; font.pixelSize: 12 }
            }
            FormationField {
                visible: root.pkgList.length > 0
                offense: root.unit === "Offense"; slots: root.pkgSlots; groups: root.unitData.groups; fits: root.fits; pkg: root.curPkg
            }
            GridLayout {
                Layout.fillWidth: true
                columns: root.width >= 1000 ? 2 : 1; columnSpacing: 12; rowSpacing: 12
                Repeater {
                    model: root.unitData.groups
                    GroupCard { g: modelData; slots: root.slotsFor(modelData); Layout.alignment: Qt.AlignTop; Layout.preferredWidth: 1 }
                }
                Repeater {
                    model: root.unit === "Special" ? (root.vm.returns || []) : []
                    GroupCard { g: modelData; slots: 1; Layout.alignment: Qt.AlignTop; Layout.preferredWidth: 1 }
                }
            }
            Item { Layout.preferredHeight: 10 }
        }
    }

    Popup {
        id: addDlg
        parent: Overlay.overlay
        anchors.centerIn: parent
        modal: true; focus: true
        width: Math.min(parent ? parent.width - 32 : 480, 480)
        height: Math.min(parent ? parent.height - 48 : 600, 600)
        padding: 0
        background: Rectangle { color: Theme.surface; radius: Theme.radiusCard + 2; border.color: Theme.outline }
        Overlay.modal: Rectangle { color: "#aa0b0b12" }
        contentItem: ColumnLayout {
            spacing: 0
            RowLayout { Layout.margins: 14
                ColumnLayout { Layout.fillWidth: true; spacing: 0
                    Text { text: root.addGroup ? "Put someone at " + root.addGroup.tag : ""; color: Theme.ink; font.bold: true; font.pixelSize: 16 }
                    Text { text: "Rated by how they would play there. Active players only."; color: Theme.inkSubtle; font.pixelSize: 12 }
                }
                PrimaryButton { text: "Close"; ghost: true; implicitHeight: 32; onClicked: addDlg.close() }
            }
            ListView {
                Layout.fillWidth: true; Layout.fillHeight: true; clip: true; model: root.candidates; spacing: 2
                ScrollBar.vertical: ScrollBar {}
                delegate: Rectangle {
                    width: ListView.view.width; height: 46; color: hva.hovered ? Theme.surfaceAlt : "transparent"
                    HoverHandler { id: hva; cursorShape: Qt.PointingHandCursor }
                    TapHandler { onTapped: root.add(modelData.id) }
                    RowLayout { anchors.fill: parent; anchors.leftMargin: 14; anchors.rightMargin: 14; spacing: 10
                        Text { text: modelData.rate; color: Theme.ovrColor(modelData.rate); font.bold: true; font.pixelSize: 15; Layout.preferredWidth: 28 }
                        Text { text: modelData.name; color: Theme.ink; font.pixelSize: 14; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { text: modelData.pos; color: Theme.inkSubtle; font.pixelSize: 12 }
                    }
                }
            }
        }
    }
}
