import QtQuick
import QtQuick.Layouts

// Your 14 games, with results as they come in.
ColumnLayout {
    id: root
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.leagueTabJson("schedule")); }
    spacing: 12
    Card {
        title: "Your 14 games"
        Repeater {
            model: root.vm.games || []
            RowLayout {
                id: gameRow
                Layout.fillWidth: true; spacing: 10
                TapHandler { enabled: modelData.box !== null; onTapped: Store.openBox(modelData.box) }
                HoverHandler { cursorShape: modelData.box !== null ? Qt.PointingHandCursor : Qt.ArrowCursor }
                Text { text: "Wk " + modelData.week; color: Theme.inkFaint; font.pixelSize: 12; Layout.preferredWidth: 40 }
                Helmet { slug: modelData.opp.helmet; size: 28 }
                Text {
                    text: (modelData.home ? "vs " : "at ") + modelData.opp.name + (modelData.rival ? "  ·  rivalry" : "")
                    color: Theme.ink; font.pixelSize: 13; Layout.fillWidth: true; elide: Text.ElideRight
                }
                Text {
                    text: modelData.result || (modelData.next ? "Next" : "")
                    color: !modelData.result ? Theme.primary : (modelData.result.charAt(0) === "W" ? Theme.goodFg : (modelData.result.charAt(0) === "L" ? Theme.badFg : Theme.inkMuted))
                    font.pixelSize: 13; font.bold: true
                }
            }
        }
    }
}
