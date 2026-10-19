import QtQuick
import QtQuick.Layouts

// The Hall of Fame: retired players whose career score cleared the bar, by class, plus the best who fell short.
ColumnLayout {
    id: root
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.leagueTabJson("hall")); }
    function awardLabel(a) {
        var p = a.split(":");
        var n = { mvp: "MVP", opoy: "Off. POY", dpoy: "Def. POY", ol: "Trench Monster" }[p[1]] || "ROY";
        return n + " " + p[0];
    }
    spacing: 12

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(width * 9 / 16, 260)
        radius: Theme.radiusCard
        color: Theme.surface
        clip: true
        Image {
            anchors.fill: parent
            source: Store.game.assetsUrl ? Store.game.assetsUrl + "artwork/stories/hall_of_fame.webp" : ""
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
        }
    }
    Text {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        color: Theme.inkSubtle; font.pixelSize: 13
        text: root.vm.count > 0
            ? root.vm.count + " member" + (root.vm.count === 1 ? "" : "s") + ". Inducted the offseason a player retires, if his career score reaches " + root.vm.bar + "."
            : "No one has retired with a Hall of Fame career yet. Players are judged when they retire: production, years as a regular, championships and awards."
    }

    component EntryRow: RowLayout {
            property var e
            property bool gold: true
            Layout.fillWidth: true
            spacing: 8
            Jersey { number: e.number; team: e.teamSlug; lastName: e.lastName; size: 76; Layout.alignment: Qt.AlignTop }
            ColumnLayout {
                Layout.fillWidth: true; spacing: 1
                Text { text: e.name; color: Theme.ink; font.pixelSize: 15; font.bold: true }
                Text {
                    text: e.pos + " · " + e.abbr + " · " + (e.firstSeason === e.lastSeason ? "Season " + e.firstSeason : "Seasons " + e.firstSeason + "-" + e.lastSeason) + " (" + e.seasons + " played)"
                    color: Theme.inkSubtle; font.pixelSize: 12
                }
                Text { text: e.line; color: Theme.inkMuted; font.pixelSize: 13; font.bold: true; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                Text {
                    property var extras: (e.titles > 0 ? [e.titles + " title" + (e.titles === 1 ? "" : "s")] : []).concat(e.awards.map(root.awardLabel))
                    visible: extras.length > 0
                    text: extras.join(" · ")
                    color: Theme.primary; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true
                }
            }
            Text { text: e.score; color: gold ? Theme.gold : Theme.inkSubtle; font.pixelSize: 16; font.bold: true; Layout.alignment: Qt.AlignTop }
        }

    Repeater {
        model: root.vm.classes || []
        Card {
            title: "Class of " + modelData.season
            Repeater {
                model: modelData.rows
                EntryRow { e: modelData }
            }
        }
    }
    Card {
        visible: (root.vm.misses || []).length > 0
        title: "Just missed"
        Repeater {
            model: root.vm.misses || []
            EntryRow { e: modelData; gold: false }
        }
    }
}
