import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    property string tab: "standings"
    Component.onCompleted: { var i = Qt.application.arguments.indexOf("--dev-ltab"); if (i >= 0) tab = Qt.application.arguments[i + 1]; }
    readonly property var tabs: [["standings", "Standings"], ["schedule", "Schedule"], ["playoffs", "Playoffs"], ["leaders", "Leaders"], ["teamstats", "Team stats"], ["awards", "Awards"], ["hall", "Hall of Fame"], ["history", "History"]]
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.standingsJson()); }

    ScrollView {
        id: sv
        anchors.fill: parent
        contentWidth: availableWidth
        clip: true
        ColumnLayout {
            width: sv.availableWidth - 2 * Theme.gutter
            x: Theme.gutter
            y: Theme.gutter
            spacing: 12
            ScreenTitle { text: "League" }
            Flow {
                Layout.fillWidth: true
                spacing: 6
                Repeater {
                    model: root.tabs
                    Chip { text: modelData[1]; selected: root.tab === modelData[0]; onClicked: root.tab = modelData[0] }
                }
            }
            Loader { Layout.fillWidth: true; active: root.tab === "schedule"; visible: active; source: "SchedulePanel.qml" }
            Loader { Layout.fillWidth: true; active: root.tab === "playoffs"; visible: active; source: "PlayoffsPanel.qml" }
            Loader { Layout.fillWidth: true; active: root.tab === "leaders"; visible: active; source: "LeadersPanel.qml" }
            Loader { Layout.fillWidth: true; active: root.tab === "teamstats"; visible: active; source: "TeamStatsPanel.qml" }
            Loader { Layout.fillWidth: true; active: root.tab === "awards"; visible: active; source: "AwardsPanel.qml" }
            Loader { Layout.fillWidth: true; active: root.tab === "hall"; visible: active; source: "HallPanel.qml" }
            Loader { Layout.fillWidth: true; active: root.tab === "history"; visible: active; source: "HistoryPanel.qml" }
            ColumnLayout {
                Layout.fillWidth: true
                visible: root.tab === "standings"
                spacing: 12

            Card {
                visible: (root.vm.seeds || []).length > 0
                title: "Playoff seeds"
                Repeater {
                    model: root.vm.seeds || []
                    RowLayout {
                        Layout.fillWidth: true; spacing: 8
                        Text { text: modelData.seed; color: Theme.inkFaint; Layout.preferredWidth: 16 }
                        Helmet { slug: modelData.team.helmet; size: 24 }
                        Text { text: modelData.team.name; color: modelData.isMe ? Theme.primary : Theme.ink; font.bold: modelData.isMe; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { text: modelData.team.record; color: Theme.inkMuted }
                    }
                }
            }

            GridLayout {
                Layout.fillWidth: true
                columns: root.width >= 1000 ? 3 : (root.width >= 640 ? 2 : 1)
                columnSpacing: 12; rowSpacing: 12
                Repeater {
                    model: root.vm.divisions || []
                    Card {
                        Layout.alignment: Qt.AlignTop
                        Layout.preferredWidth: 1
                        title: modelData.name
                        Repeater {
                            model: modelData.rows
                            RowLayout {
                                Layout.fillWidth: true; spacing: 8
                                Text { text: modelData.rank; color: Theme.inkFaint; Layout.preferredWidth: 14; font.pixelSize: 13 }
                                Helmet { slug: modelData.team.helmet; size: 24 }
                                Text { text: modelData.team.name; color: modelData.isMe ? Theme.primary : Theme.ink; font.bold: modelData.isMe; font.pixelSize: 13; Layout.fillWidth: true; elide: Text.ElideRight }
                                Text { text: modelData.record; color: Theme.inkMuted; font.pixelSize: 13; font.bold: true }
                                Text { text: modelData.pf + "-" + modelData.pa; color: Theme.inkSubtle; font.pixelSize: 11; Layout.preferredWidth: 56; horizontalAlignment: Text.AlignRight }
                            }
                        }
                    }
                }
            }

            Card {
                visible: (root.vm.recent || []).length > 0
                title: "Recent results"
                Repeater {
                    model: root.vm.recent || []
                    RowLayout {
                        Layout.fillWidth: true; spacing: 8
                        Text { text: "Wk " + modelData.week; color: Theme.inkFaint; font.pixelSize: 12; Layout.preferredWidth: 38 }
                        Text { text: modelData.away.name + "  " + modelData.as + "  @  " + modelData.hs + "  " + modelData.home.name + (modelData.ot ? " (OT)" : ""); color: modelData.mine ? Theme.primary : Theme.inkMuted; font.bold: modelData.mine; font.pixelSize: 13; Layout.fillWidth: true; elide: Text.ElideRight }
                    }
                }
            }

            Card {
                visible: (root.vm.history || []).length > 0
                title: "Champions"
                Repeater { model: root.vm.history || []; Text { text: "Season " + modelData.season + "  —  " + modelData.champion; color: Theme.ink; font.pixelSize: 13 } }
            }
            }
            Item { Layout.preferredHeight: 8 }
        }
    }
}
