import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The owner's career: season by season, honors, awards, Hall of Famers and the clubs they helped. Saved as a picture.
Item {
    id: root
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.legacyJson()); }
    readonly property bool empty: !vm.count
    property string said: ""
    Timer { id: sayTimer; interval: 5000; onTriggered: root.said = "" }

    function finishColor(s) { return s.champion ? "#e8b931" : (s.runnerUp ? "#b9c2bc" : (!s.madePlayoffs ? "#4a5170" : Theme.goodFg)); }
    function saveCard() {
        var slug = (vm.team || "team").toLowerCase().replace(/[^a-z0-9]+/g, "-");
        var path = Store.game.downloadsDir() + "/semi-pro-legacy-" + slug + ".png";
        card.grabToImage(function(r) {
            root.said = r.saveToFile(path) ? "Saved the picture to " + path : "Could not make the picture. Try again.";
            sayTimer.restart();
        }, Qt.size(1080, 1350));
    }

    component Big: ColumnLayout {
        property string value; property string name; property color tint: "white"
        spacing: 4
        Text { text: value; color: tint; font.pixelSize: 30; font.bold: true; Layout.alignment: Qt.AlignHCenter; fontSizeMode: Text.Fit; minimumPixelSize: 14; Layout.maximumWidth: 140 }
        Text { text: name; color: "#bfc9e8"; font.pixelSize: 10; font.bold: true; font.letterSpacing: 1.1; Layout.alignment: Qt.AlignHCenter }
    }
    component Small: ColumnLayout {
        property string value; property string name
        Layout.fillWidth: true; Layout.preferredWidth: 1; Layout.maximumWidth: 10000
        spacing: 3
        Text { text: value; color: "white"; font.pixelSize: 22; font.bold: true; Layout.alignment: Qt.AlignHCenter }
        Text { text: name; color: "#bfc9e8"; font.pixelSize: 9; font.bold: true; font.letterSpacing: 1.1; horizontalAlignment: Text.AlignHCenter; Layout.alignment: Qt.AlignHCenter }
    }

    ScrollView {
        id: sv
        anchors.fill: parent
        contentWidth: availableWidth
        clip: true
        ColumnLayout {
            width: Math.min(sv.availableWidth - 2 * Theme.gutter, 900)
            x: Math.max(Theme.gutter, (sv.availableWidth - width) / 2)
            y: Theme.gutter
            spacing: 12
            RowLayout {
                spacing: 12
                Helmet { slug: root.vm.helmet || ""; size: 56 }
                ColumnLayout {
                    spacing: 0
                    ScreenTitle { text: "Legacy" }
                    Text { text: root.empty ? "Your story starts after the first season." : root.vm.team + " · " + root.vm.count + " season" + (root.vm.count === 1 ? "" : "s") + " as owner"; color: Theme.inkSubtle; font.pixelSize: 13 }
                }
            }
            Card {
                visible: root.empty
                Text { text: "Nothing to show yet. Finish a season and your record, titles and good deeds will collect here."; color: Theme.inkMuted; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            }

            Card {
                visible: !root.empty
                title: "Share it"
                Item {
                    Layout.alignment: Qt.AlignHCenter
                    implicitWidth: 420; implicitHeight: 525
                    Rectangle {
                        id: card
                        width: 420; height: 525
                        gradient: Gradient {
                            GradientStop { position: 0; color: "#101a33" }
                            GradientStop { position: 0.5; color: "#26396b" }
                            GradientStop { position: 1; color: "#16264a" }
                        }
                        ColumnLayout {
                            anchors.fill: parent; anchors.margins: 26; anchors.topMargin: 24; spacing: 0
                            Image {
                                Layout.alignment: Qt.AlignHCenter
                                Layout.preferredHeight: 56
                                source: Store.game.assetsUrl + "artwork/semi-pro-football-splash-logo.png"
                                fillMode: Image.PreserveAspectFit; sourceSize.height: 168; asynchronous: true
                            }
                            Item { Layout.preferredHeight: 12 }
                            RowLayout {
                                Layout.fillWidth: true; spacing: 12
                                Helmet { slug: root.vm.helmet || ""; size: 58 }
                                ColumnLayout {
                                    Layout.fillWidth: true; spacing: 4
                                    Text { text: (root.vm.team || "").toUpperCase(); color: "white"; font.pixelSize: 25; font.bold: true; wrapMode: Text.WordWrap; Layout.fillWidth: true; maximumLineCount: 2 }
                                    Text { text: "OWNER LEGACY · " + root.vm.count + " SEASON" + (root.vm.count === 1 ? "" : "S"); color: "#bfc9e8"; font.pixelSize: 10; font.bold: true; font.letterSpacing: 1.1 }
                                }
                            }
                            Item { Layout.preferredHeight: 16 }
                            Rectangle {
                                Layout.fillWidth: true; implicitHeight: 78; radius: Theme.radiusCard; color: "#14ffffff"
                                RowLayout {
                                    anchors.fill: parent; anchors.margins: 8
                                    Big { Layout.fillWidth: true; Layout.preferredWidth: 5; value: "" + root.vm.titles; name: root.vm.titles === 1 ? "CHAMPIONSHIP" : "CHAMPIONSHIPS"; tint: "#e8b931" }
                                    Big { Layout.fillWidth: true; Layout.preferredWidth: 6; value: root.vm.record || ""; name: "RECORD" }
                                    Big { Layout.fillWidth: true; Layout.preferredWidth: 5; value: (root.vm.pct || "") + "%"; name: "WIN RATE" }
                                }
                            }
                            Item { Layout.preferredHeight: 16 }
                            RowLayout {
                                Layout.fillWidth: true; Layout.maximumWidth: 10000
                                Small { Layout.fillWidth: true; value: "" + root.vm.divisionTitles; name: "DIVISION\nTITLES" }
                                Small { Layout.fillWidth: true; value: "" + root.vm.playoffTrips; name: "PLAYOFF\nTRIPS" }
                                Small { Layout.fillWidth: true; value: root.vm.playoffRecord || ""; name: "PLAYOFF\nRECORD" }
                            }
                            Item { Layout.preferredHeight: 14 }
                            RowLayout {
                                Layout.fillWidth: true; Layout.maximumWidth: 10000
                                Small { Layout.fillWidth: true; value: "" + (root.vm.awards || []).length; name: "PLAYER\nAWARDS" }
                                Small { Layout.fillWidth: true; value: "" + (root.vm.hof || []).length; name: "HALL OF\nFAMERS" }
                                Small { Layout.fillWidth: true; value: "" + root.vm.clubsHelped; name: "CLUBS\nHELPED" }
                            }
                            Item { Layout.preferredHeight: 18 }
                            Flow {
                                Layout.fillWidth: true; spacing: 6
                                layoutDirection: Qt.LeftToRight
                                Repeater {
                                    model: (root.vm.seasons || []).slice(0, 14).reverse()
                                    Rectangle {
                                        width: 20; height: 20; radius: 10; color: root.finishColor(modelData)
                                        Text { visible: modelData.champion; anchors.centerIn: parent; text: "★"; color: "#3a2a00"; font.pixelSize: 12 }
                                    }
                                }
                            }
                            Item { Layout.fillHeight: true }
                            Text { text: "SEMI-PRO FOOTBALL  ·  #SEMIPROFOOTBALL"; color: "#bfc9e8"; font.pixelSize: 9; font.bold: true; font.letterSpacing: 1.4; Layout.alignment: Qt.AlignHCenter }
                        }
                    }
                }
                PrimaryButton { text: "Save the picture"; Layout.alignment: Qt.AlignHCenter; implicitHeight: 40; onClicked: root.saveCard() }
                Text { visible: root.said !== ""; text: root.said; color: Theme.primary; font.pixelSize: 12; wrapMode: Text.WrapAnywhere; Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter }
            }

            Card {
                visible: !root.empty
                title: "Career"
                GridLayout {
                    Layout.fillWidth: true; rowSpacing: 12; columnSpacing: 12
                    columns: root.width > 760 ? 3 : (root.width > 480 ? 2 : 1)
                    uniformCellWidths: true
                    StatTile { label: "Record"; value: (root.vm.record || "") + "  ·  " + root.vm.pct + "%" }
                    StatTile { label: "Championships"; value: "" + root.vm.titles }
                    StatTile { label: "Division titles"; value: "" + root.vm.divisionTitles }
                    StatTile { label: "Playoff trips"; value: root.vm.playoffTrips + " of " + root.vm.count }
                    StatTile { label: root.vm.playoffExact ? "Playoff record" : "Playoff record (older seasons partial)"; value: root.vm.playoffRecord || "" }
                    StatTile { visible: !!root.vm.best; label: "Best season"; value: root.vm.best ? root.vm.best.record + "  ·  S" + root.vm.best.season : "" }
                }
            }
            Card {
                visible: !root.empty
                title: "Season by season"
                Repeater {
                    model: root.vm.seasons || []
                    RowLayout {
                        Layout.fillWidth: true; spacing: 8
                        Text { text: "S" + modelData.season; color: Theme.ink; font.bold: true; Layout.preferredWidth: 44 }
                        Text { text: modelData.record; color: Theme.ink; font.bold: true; Layout.preferredWidth: 62 }
                        Rectangle { width: 10; height: 10; radius: 5; color: root.finishColor(modelData) }
                        Text { text: modelData.runnerUp ? "" : modelData.finish; color: Theme.inkMuted; font.pixelSize: 13; Layout.fillWidth: true }
                        Text { visible: modelData.divisionTitle; text: "⚑"; color: Theme.primary; font.pixelSize: 16 }
                        Text { visible: modelData.champion; text: "🏆"; font.pixelSize: 16 }
                        Text { visible: modelData.playoffs !== null; text: modelData.playoffs || ""; color: Theme.inkMuted; font.pixelSize: 13 }
                    }
                }
            }
            Card {
                visible: !root.empty
                title: "Awards won by your players (" + (root.vm.awards || []).length + ")"
                Text { visible: (root.vm.awards || []).length === 0; text: "No award winners yet."; color: Theme.inkMuted; font.pixelSize: 13 }
                Repeater {
                    model: root.vm.awards || []
                    RowLayout {
                        Layout.fillWidth: true; spacing: 8
                        Text { text: "🎖"; font.pixelSize: 16 }
                        Text { text: modelData.text; color: Theme.ink; font.bold: true; font.pixelSize: 13; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { text: "S" + modelData.season; color: Theme.inkSubtle; font.pixelSize: 12 }
                    }
                }
            }
            Card {
                visible: !root.empty
                title: "Hall of Famers (" + (root.vm.hof || []).length + ")"
                Text { visible: (root.vm.hof || []).length === 0; text: "Nobody has been inducted from your club yet."; color: Theme.inkMuted; font.pixelSize: 13 }
                Repeater {
                    model: root.vm.hof || []
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 0
                        Text { text: modelData.title; color: Theme.ink; font.bold: true; font.pixelSize: 14 }
                        Text { text: modelData.line; color: Theme.inkSubtle; font.pixelSize: 12 }
                    }
                }
            }
            Card {
                visible: !root.empty
                title: "Clubs you helped"
                Text { visible: (root.vm.help || []).length === 0; text: "You have not bailed out another club yet. When a struggling club writes, you can give from your own cash."; color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                RowLayout {
                    visible: (root.vm.help || []).length > 0
                    spacing: 12
                    StatTile { label: "Clubs"; value: "" + root.vm.clubsHelped; implicitWidth: 120 }
                    StatTile { label: "Total given (" + root.vm.gifts + " gift" + (root.vm.gifts === 1 ? "" : "s") + ")"; value: root.vm.totalGiven || ""; implicitWidth: 200 }
                }
                Repeater {
                    model: root.vm.help || []
                    RowLayout {
                        Layout.fillWidth: true; spacing: 12
                        Text { text: modelData.team; color: Theme.ink; font.bold: true; font.pixelSize: 13; Layout.fillWidth: true }
                        Text { text: "S" + modelData.season; color: Theme.inkSubtle; font.pixelSize: 12 }
                        Text { text: modelData.amount; color: Theme.ink; font.bold: true; font.pixelSize: 13 }
                    }
                }
            }
            Item { Layout.preferredHeight: 8 }
        }
    }
}
