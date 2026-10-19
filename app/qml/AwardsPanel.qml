import QtQuick
import QtQuick.Layouts

// MVP, Offensive / Defensive Player of the Year, Trench Monster, Rookie of the Year: top three each. A race until the
// regular season ends, final after. Past winners below.
ColumnLayout {
    id: root
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.leagueTabJson("awards")); }
    readonly property var medal: ["#d9a400", "#8e98a3", "#b36b2d"]
    function band(id) {
        return id === "mvp" ? "#8a6100" : (id === "opoy" ? "#1e5b3a" : (id === "dpoy" ? "#1f4e79" : (id === "ol" ? "#5a3d7a" : "#9a5b1e")));
    }
    spacing: 12

    Text {
        text: (root.vm.locked ? "✔  " : "◔  ") + (root.vm.status || "")
        color: root.vm.locked ? Theme.primary : Theme.inkSubtle
        font.pixelSize: 14; font.bold: true
    }

    Repeater {
        model: root.vm.categories || []
        Rectangle {
            id: cat
            readonly property color bandColor: root.band(modelData.id)
            Layout.fillWidth: true
            implicitHeight: cc.implicitHeight
            radius: Theme.radiusCard
            color: Theme.surface
            border.color: Qt.alpha(bandColor, 0.6)
            clip: true
            ColumnLayout {
                id: cc
                width: parent.width
                spacing: 0
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: hdr.implicitHeight + 20
                    color: cat.bandColor
                    ColumnLayout {
                        id: hdr
                        anchors.fill: parent; anchors.margins: 10; anchors.leftMargin: 14
                        spacing: 0
                        Text { text: modelData.title.toUpperCase(); color: "white"; font.pixelSize: 20; font.bold: true; font.letterSpacing: 1 }
                        Text { text: modelData.blurb; color: "#efe6d2"; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    }
                }
                Text { visible: modelData.finalists.length === 0; text: "No finalists yet."; color: Theme.inkSubtle; leftPadding: 14; topPadding: 12; bottomPadding: 12 }
                Repeater {
                    model: modelData.finalists
                    Rectangle {
                        required property var modelData
                        required property int index
                        Layout.fillWidth: true
                        implicitHeight: fr.implicitHeight + 24
                        color: index === 0 ? Qt.alpha(cat.bandColor, 0.14) : (rowHover.hovered ? Theme.surfaceHi : "transparent")
                        Rectangle { visible: index > 0; width: parent.width; height: 1; color: Theme.divider }
                        RowLayout {
                            id: fr
                            anchors.fill: parent; anchors.margins: 12; anchors.leftMargin: 14
                            spacing: 12
                            Rectangle {
                                width: 32; height: 32; radius: 16; color: root.medal[index]
                                Text { anchors.centerIn: parent; text: index + 1; color: "white"; font.bold: true; font.pixelSize: 15 }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 2
                                RowLayout {
                                    spacing: 8
                                    Text { text: modelData.name; color: Theme.ink; font.pixelSize: index === 0 ? 17 : 15; font.bold: true }
                                    Pill { visible: index === 0 && root.vm.locked; text: "WINNER"; fg: "white"; bg: cat.bandColor }
                                    Pill { visible: modelData.mine; text: "YOUR PLAYER"; fg: Theme.primary; bg: Theme.primaryTint }
                                }
                                Text { text: modelData.pos + " · " + modelData.team; color: Theme.inkSubtle; font.pixelSize: 12 }
                                Text { text: modelData.line; color: Theme.inkMuted; font.pixelSize: 13; font.bold: true; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                            }
                            Helmet { slug: modelData.helmet; size: 40 }
                        }
                        HoverHandler { id: rowHover; cursorShape: Qt.PointingHandCursor }
                        TapHandler { onTapped: Store.openPlayer(modelData.playerId) }
                    }
                }
            }
        }
    }

    Card {
        visible: (root.vm.past || []).length > 0
        title: "Past winners"
        Repeater {
            model: root.vm.past || []
            ColumnLayout {
                Layout.fillWidth: true; spacing: 2
                Text { text: "SEASON " + modelData.season; color: Theme.inkSubtle; font.pixelSize: 11; font.bold: true; font.letterSpacing: 1.1; topPadding: 6 }
                Repeater {
                    model: modelData.winners
                    RowLayout {
                        Layout.fillWidth: true; spacing: 8
                        Text { text: modelData.id.toUpperCase(); color: Theme.inkSubtle; font.pixelSize: 12; font.bold: true; Layout.preferredWidth: 44 }
                        Text { text: modelData.text; color: Theme.ink; font.pixelSize: 13; Layout.fillWidth: true; elide: Text.ElideRight }
                    }
                }
            }
        }
    }
}
