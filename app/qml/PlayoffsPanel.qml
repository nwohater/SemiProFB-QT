import QtQuick
import QtQuick.Layouts

// Seeds and the bracket. Set after week 14: three division winners and one wild card.
ColumnLayout {
    id: root
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.leagueTabJson("playoffs")); }
    spacing: 12
    Card {
        visible: !root.vm.set
        Text { text: "The bracket is set after week 14: three division winners and one wild card."; color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
    }
    Card {
        visible: !!root.vm.set
        title: "Seeds"
        Repeater {
            model: root.vm.seeds || []
            RowLayout {
                Layout.fillWidth: true; spacing: 8
                Text { text: modelData.seed; color: Theme.inkFaint; Layout.preferredWidth: 20 }
                Helmet { slug: modelData.team.helmet; size: 28 }
                Text { text: modelData.team.name; color: modelData.isMe ? Theme.primary : Theme.ink; font.bold: modelData.isMe; Layout.fillWidth: true; elide: Text.ElideRight }
                Text { text: modelData.team.record; color: Theme.inkMuted }
            }
        }
    }
    Card {
        visible: !!root.vm.set
        title: "Bracket"
        Repeater {
            model: root.vm.games || []
            RowLayout {
                Layout.fillWidth: true; spacing: 8
                Text { text: modelData.label; color: Theme.inkFaint; font.pixelSize: 12; Layout.preferredWidth: 76 }
                Helmet { slug: modelData.away.helmet; size: 26 }
                Text {
                    text: modelData.away.name + " (" + modelData.awaySeed + ") at " + modelData.home.name + " (" + modelData.homeSeed + ")"
                    color: Theme.ink; font.pixelSize: 13; Layout.fillWidth: true; elide: Text.ElideRight
                }
                Helmet { slug: modelData.home.helmet; size: 26 }
                Text { text: modelData.score || ""; color: Theme.ink; font.bold: true; font.pixelSize: 13 }
            }
        }
        Text {
            visible: !!root.vm.champion
            text: "Champions: " + root.vm.champion
            color: Theme.gold; font.bold: true; font.pixelSize: 14
        }
    }
}
