import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The weekly league paper: one issue per week played.
Item {
    id: root
    property int picked: -1          // -1 follows the newest issue
    property int swapIssue: -1       // the issue the swap below belongs to
    property int swapBrief: -1       // index (among briefs) that has taken over the front page
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.newsJson(picked)); }
    readonly property var stories: vm.stories || []
    function inSection(sec) { return stories.filter(function(s) { return s.section === sec; }); }
    readonly property var briefs0: inSection("Briefs")
    readonly property var front: inSection("Front")
    readonly property bool swapped: !vm.empty && swapIssue === vm.idx && front.length > 0 && swapBrief >= 0 && swapBrief < briefs0.length
    readonly property var lead: swapped ? briefs0[swapBrief] : (front.length > 0 ? front[0] : null)
    readonly property var briefs: briefs0.map(function(b, i) { return i === swapBrief && swapped ? front[0] : b; })
    function art(kind) { return Store.game.assetsUrl + "artwork/stories/" + kind + ".webp"; }

    component StoryText: ColumnLayout {
        property var s
        spacing: 2
        Text { text: s.headline; color: Theme.ink; font.pixelSize: 15; font.bold: true; wrapMode: Text.WordWrap; Layout.fillWidth: true }
        Text { visible: s.body !== ""; text: s.body; color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
    }
    component SectionCard: Card {
        id: sc
        property string sec: ""
        property string heading: ""
        property Component row
        title: heading
        visible: root.inSection(sec).length > 0
        Repeater {
            model: root.inSection(sc.sec)
            Loader {
                Layout.fillWidth: true
                sourceComponent: sc.row
                onLoaded: item.s = modelData
            }
        }
    }

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

            // masthead
            RowLayout {
                Layout.fillWidth: true
                spacing: 12
                ColumnLayout {
                    spacing: 0
                    ScreenTitle { text: (root.vm.region || "") + " Gazette" }
                    Text {
                        text: root.vm.empty ? "Semi-pro football, covered weekly" : "Season " + root.vm.season + " · " + root.vm.label
                        color: Theme.inkSubtle; font.pixelSize: 13
                    }
                }
                Item { Layout.fillWidth: true }
                RowLayout {
                    visible: !root.vm.empty && root.vm.count > 1
                    spacing: 6
                    PrimaryButton { text: "‹"; ghost: true; implicitHeight: 34; leftPadding: 12; rightPadding: 12; enabled: root.vm.idx > 0; onClicked: root.picked = root.vm.idx - 1 }
                    Text { text: "Issue " + root.vm.number; color: Theme.ink; font.bold: true; font.pixelSize: 14 }
                    PrimaryButton { text: "›"; ghost: true; implicitHeight: 34; leftPadding: 12; rightPadding: 12; enabled: root.vm.idx < root.vm.count - 1; onClicked: root.picked = (root.vm.idx + 1 === root.vm.count - 1) ? -1 : root.vm.idx + 1 }
                }
            }

            Card {
                visible: !!root.vm.empty
                Text { text: "The first issue prints after the first week of games."; color: Theme.inkMuted; font.pixelSize: 13 }
            }

            // front page
            Card {
                visible: root.lead !== null
                TapHandler { enabled: !!root.lead && (root.lead.box !== null || root.lead.playerId !== null); onTapped: { if (root.lead.box !== null) Store.openBox(root.lead.box); else Store.openPlayer(root.lead.playerId); } }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 20
                    Image {
                        visible: !!root.lead && root.lead.kind !== "" && status !== Image.Error
                        source: root.lead && root.lead.kind !== "" ? root.art(root.lead.kind) : ""
                        Layout.preferredWidth: Math.min(380, sv.availableWidth * 0.4)
                        Layout.preferredHeight: Layout.preferredWidth * 9 / 16
                        Layout.alignment: Qt.AlignTop
                        fillMode: Image.PreserveAspectCrop
                        asynchronous: true
                    }
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 8
                        Text { text: root.lead ? root.lead.headline : ""; color: Theme.ink; font.pixelSize: 26; font.bold: true; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                        Text { text: root.lead ? root.lead.body : ""; color: Theme.inkMuted; font.pixelSize: 15; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                        PrimaryButton {
                            visible: !!root.lead && root.lead.playerId !== null
                            text: "Open player"; ghost: true; implicitHeight: 32
                            onClicked: Store.openPlayer(root.lead.playerId)
                        }
                    }
                }
            }

            // briefs: tap to bring one to the front page
            Card {
                visible: root.briefs.length > 0
                title: "Also in the news"
                Repeater {
                    model: root.briefs
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: br.implicitHeight + 16
                        color: bh.containsMouse ? Theme.surfaceHi : "transparent"
                        radius: 8
                        MouseArea {
                            id: bh
                            anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                root.swapIssue = root.vm.idx;
                                root.swapBrief = (root.swapped && root.swapBrief === index) ? -1 : index;
                            }
                        }
                        RowLayout {
                            id: br
                            anchors.fill: parent; anchors.margins: 8
                            spacing: 12
                            Image {
                                visible: modelData.kind !== "" && status !== Image.Error
                                source: modelData.kind !== "" ? root.art(modelData.kind) : ""
                                Layout.preferredWidth: 132; Layout.preferredHeight: 99
                                fillMode: Image.PreserveAspectCrop; asynchronous: true
                            }
                            Helmet { visible: modelData.kind === "" && !!modelData.helmet; slug: modelData.helmet || ""; size: 52 }
                            StoryText { Layout.fillWidth: true; s: modelData }
                            PrimaryButton {
                                visible: modelData.playerId !== null
                                text: "Player ›"; ghost: true; implicitHeight: 28; font.pixelSize: 12; leftPadding: 10; rightPadding: 10
                                onClicked: Store.openPlayer(modelData.playerId)
                            }
                            Text { text: "⇅"; color: Theme.inkFaint; font.pixelSize: 16 }
                        }
                    }
                }
            }

            // scoreboard
            SectionCard {
                sec: "Scores"; heading: "Scoreboard"
                row: Component {
                    ColumnLayout {
                        property var s
                        spacing: 4
                        TapHandler { enabled: s.box !== null; onTapped: Store.openBox(s.box) }
                        Text { visible: !!s.game && s.game.tag !== ""; text: s.game ? s.game.tag.toUpperCase() : ""; color: Theme.primary; font.pixelSize: 11; font.bold: true; font.letterSpacing: 0.6 }
                        Repeater {
                            model: s.game ? [ { t: s.game.away, sc: s.game.as, won: s.game.as > s.game.hs }, { t: s.game.home, sc: s.game.hs, won: s.game.hs > s.game.as } ] : []
                            RowLayout {
                                Layout.fillWidth: true; spacing: 10
                                Helmet { slug: modelData.t.helmet; size: 30 }
                                Text { text: modelData.t.name; color: Theme.ink; font.pixelSize: 15; font.bold: modelData.won; Layout.fillWidth: true; elide: Text.ElideRight }
                                Text { text: modelData.sc; color: Theme.ink; font.pixelSize: 18; font.bold: modelData.won }
                            }
                        }
                        Text { visible: s.body !== ""; text: s.body; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider }
                    }
                }
            }
            // helmet / jersey rows
            Repeater {
                model: [ ["Spotlight", "Player spotlight"], ["Absences", "Out this week"], ["Money", "Business page"], ["Gossip", "Gossip"] ]
                SectionCard {
                    sec: modelData[0]; heading: modelData[1]
                    row: Component {
                        Item {
                            id: srow
                            property var s
                            implicitHeight: rl.implicitHeight
                            RowLayout {
                                id: rl
                                width: parent.width
                                spacing: 12
                                Jersey { visible: !!srow.s.jersey; number: srow.s.jersey ? srow.s.jersey.number : 0; team: srow.s.jersey ? srow.s.jersey.teamSlug : ""; lastName: srow.s.jersey ? srow.s.jersey.lastName : ""; size: 68; Layout.alignment: Qt.AlignTop }
                                Helmet { visible: !srow.s.jersey && !!srow.s.helmet; slug: srow.s.helmet || ""; size: 44; Layout.alignment: Qt.AlignTop }
                                StoryText { Layout.fillWidth: true; s: srow.s }
                                Text { visible: srow.s.playerId !== null; text: "›"; color: Theme.inkFaint; font.pixelSize: 18 }
                            }
                            MouseArea {
                                anchors.fill: parent
                                enabled: srow.s.playerId !== null; cursorShape: srow.s.playerId !== null ? Qt.PointingHandCursor : Qt.ArrowCursor
                                onClicked: Store.openPlayer(srow.s.playerId)
                            }
                        }
                    }
                }
            }
            SectionCard {
                sec: "Standings"; heading: "Around the league"
                row: Component { StoryText {} }
            }
            Item { Layout.preferredHeight: 8 }
        }
    }
}
