import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Pick a region, see the 12 dealt towns (4 per division), choose your club, optionally swap its nickname.
Item {
    id: root
    signal back()
    signal started()

    property string regionId: "ca"
    property var regions: JSON.parse(Store.game.regionsJson())
    property var towns: []
    property var nicks: ({})          // town id -> chosen nickname
    property string selectedId: ""
    property var nickPool: []
    readonly property var selected: { for (var i = 0; i < towns.length; i++) if (towns[i].id === selectedId) return towns[i]; return null; }
    readonly property bool narrow: width < 760

    function reshuffle() {
        var d = JSON.parse(Store.game.dealJson(regionId, 0));
        towns = d.towns;
        var n = {};
        for (var i = 0; i < d.towns.length; i++) n[d.towns[i].id] = d.towns[i].nickname;
        nicks = n;
        selectedId = d.towns[0].id;
        nickPool = JSON.parse(Store.game.nicknamesJson(regionId));
    }
    function setRegion(id) { regionId = id; reshuffle(); }
    function nickOf(t) { return nicks[t.id] || t.nickname; }
    function takenByOthers(name) {
        for (var k in nicks) if (k !== selectedId && nicks[k] === name) return true;
        return false;
    }
    function start() {
        var ids = []; for (var i = 0; i < towns.length; i++) ids.push(towns[i].id);
        if (Store.game.newGame(regionId, JSON.stringify(ids), selectedId, JSON.stringify(nicks))) root.started();
    }
    Component.onCompleted: reshuffle()

    ScrollView {
        id: scroll
        anchors.fill: parent
        anchors.bottomMargin: bar.height
        contentWidth: availableWidth
        clip: true
        ColumnLayout {
            x: Math.max(Theme.gutter, (scroll.availableWidth - width) / 2)
            width: Math.min(scroll.availableWidth - Theme.gutter * 2, 1100)
            spacing: 14

            Item { Layout.preferredHeight: 6 }
            RowLayout {
                Layout.fillWidth: true
                PrimaryButton { text: "‹ Back"; ghost: true; onClicked: root.back() }
                ScreenTitle { text: "New game"; Layout.fillWidth: true; Layout.leftMargin: 8 }
            }
            Flow {
                Layout.fillWidth: true
                spacing: 8
                Repeater {
                    model: root.regions
                    Chip { text: modelData.name; selected: root.regionId === modelData.id; onClicked: root.setRegion(modelData.id) }
                }
                Chip { text: "↻ Deal different towns"; onClicked: root.reshuffle() }
            }
            Text {
                Layout.fillWidth: true
                text: "Twelve towns, four per division. Pick the one you will own. Bigger, richer, more loyal towns are easier to run."
                color: Theme.inkSubtle; font.pixelSize: 13; wrapMode: Text.WordWrap
            }

            GridLayout {
                Layout.fillWidth: true
                columns: root.narrow ? 1 : 2
                columnSpacing: 14
                rowSpacing: 14

                // the towns, by division
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignTop
                    spacing: 12
                    Repeater {
                        model: root.regions.length ? (function() { for (var i = 0; i < root.regions.length; i++) if (root.regions[i].id === root.regionId) return root.regions[i].divisions; return []; })() : []
                        Card {
                            title: modelData + " division"
                            Repeater {
                                model: root.towns.filter(function(t) { return t.division === modelData; })
                                Rectangle {
                                    Layout.fillWidth: true
                                    implicitHeight: 56
                                    radius: Theme.radiusChip + 2
                                    color: root.selectedId === modelData.id ? Theme.primaryTint : (hov.hovered ? Theme.surfaceHi : Theme.surfaceAlt)
                                    border.color: root.selectedId === modelData.id ? Theme.primary : "transparent"
                                    border.width: 2
                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.margins: 8
                                        spacing: 10
                                        Helmet { slug: root.nickOf(modelData).toLowerCase().replace(/ /g, "-"); size: 40 }
                                        ColumnLayout {
                                            Layout.fillWidth: true
                                            spacing: 0
                                            Text { text: modelData.name + " " + root.nickOf(modelData); color: Theme.ink; font.bold: true; font.pixelSize: 14; elide: Text.ElideRight; Layout.fillWidth: true }
                                            Text { text: modelData.quirk; color: Theme.inkSubtle; font.pixelSize: 12; elide: Text.ElideRight; Layout.fillWidth: true }
                                        }
                                        Pill {
                                            text: modelData.difficulty
                                            fg: modelData.difficulty === "Easy" ? Theme.goodFg : (modelData.difficulty === "Hard" ? Theme.badFg : Theme.warnFg)
                                            bg: modelData.difficulty === "Easy" ? Theme.goodBg : (modelData.difficulty === "Hard" ? Theme.badBg : Theme.warnBg)
                                        }
                                    }
                                    HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
                                    TapHandler { onTapped: root.selectedId = modelData.id }
                                }
                            }
                        }
                    }
                }

                // the selected town
                Card {
                    Layout.alignment: Qt.AlignTop
                    Layout.fillWidth: true
                    title: "Your club"
                    visible: root.selected !== null
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 14
                        StateMap {
                            Layout.preferredWidth: 190; Layout.preferredHeight: 240; Layout.alignment: Qt.AlignTop
                            region: root.regionId; towns: root.towns; selectedId: root.selectedId
                            division: root.selected ? root.selected.division : ""
                            onPicked: function(id) { root.selectedId = id }
                        }
                        Helmet { slug: root.selected ? root.nickOf(root.selected).toLowerCase().replace(/ /g, "-") : ""; size: 96 }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Text { text: root.selected ? root.selected.name : ""; color: Theme.ink; font.pixelSize: 24; font.bold: true }
                            Text { text: root.selected ? root.nickOf(root.selected) : ""; color: Theme.primary; font.pixelSize: 18; font.bold: true }
                            Text { text: root.selected ? "Mascot: " + root.selected.mascot : ""; color: Theme.inkSubtle; font.pixelSize: 13 }
                            Text { text: root.selected ? root.selected.quirk : ""; color: Theme.inkMuted; font.pixelSize: 13; font.italic: true }
                        }
                    }
                    GridLayout {
                        Layout.fillWidth: true
                        columns: 3
                        rowSpacing: 8; columnSpacing: 8
                        StatTile { label: "Population"; value: root.selected ? String(root.selected.population).replace(/\B(?=(\d{3})+(?!\d))/g, ",") : "" }
                        StatTile { label: "Stadium"; value: root.selected ? String(root.selected.capacity).replace(/\B(?=(\d{3})+(?!\d))/g, ",") : "" }
                        StatTile { label: "Difficulty"; value: root.selected ? root.selected.difficulty : ""; accent: root.selected && root.selected.difficulty === "Easy" ? Theme.goodFg : (root.selected && root.selected.difficulty === "Hard" ? Theme.badFg : Theme.warnFg) }
                        StatTile { label: "Wealth"; value: root.selected ? root.selected.wealth + " / 10" : "" }
                        StatTile { label: "Loyalty"; value: root.selected ? root.selected.loyalty + " / 10" : "" }
                        StatTile { label: "Business"; value: root.selected ? root.selected.biz + " / 10" : "" }
                    }
                    Text { text: "Nickname"; color: Theme.inkSubtle; font.pixelSize: 12; font.bold: true }
                    ComboBox {
                        id: nickBox
                        Layout.fillWidth: true
                        model: root.nickPool.filter(function(n) { return !root.takenByOthers(n.name); }).map(function(n) { return n.name; })
                        currentIndex: root.selected ? Math.max(0, model.indexOf(root.nickOf(root.selected))) : 0
                        onActivated: { var n = JSON.parse(JSON.stringify(root.nicks)); n[root.selectedId] = currentText; root.nicks = n; }
                    }
                }
            }
            Item { Layout.preferredHeight: 12 }
        }
    }

    Rectangle {
        id: bar
        anchors.bottom: parent.bottom
        width: parent.width
        height: 66
        color: Theme.bg
        Rectangle { width: parent.width; height: 1; color: Theme.divider }
        RowLayout {
            anchors.fill: parent
            anchors.margins: 12
            Text { text: root.selected ? "Own the " + root.selected.name + " " + root.nickOf(root.selected) : ""; color: Theme.inkMuted; font.pixelSize: 14; Layout.fillWidth: true; elide: Text.ElideRight }
            PrimaryButton { text: "Start season 1 ▶"; onClicked: root.start() }
        }
    }
}
