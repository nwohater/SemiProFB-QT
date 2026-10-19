import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The only game view: no live play-by-play. Summary, Teams, Players, Drives.
Popup {
    id: dlg
    property int index: -1
    property string tab: "Summary"
    property string side: "home"
    property string pcat: "passing"
    readonly property var b: { Store.game.rev; return index < 0 ? ({}) : JSON.parse(Store.game.boxScoreJson(index)); }
    readonly property var catKeys: [["passing", "Passing"], ["rushing", "Rushing"], ["receiving", "Receiving"], ["offensiveLine", "Offensive line"], ["defense", "Defense"], ["kicking", "Kicking"], ["punting", "Punting"], ["returns", "Returns"]]
    function show(i) { index = i; tab = "Summary"; var a = Qt.application.arguments.indexOf("--dev-btab"); if (a >= 0) tab = Qt.application.arguments[a + 1]; side = (b && b.mySide) || "home"; pcat = "passing"; open(); }

    modal: true
    focus: true
    anchors.centerIn: parent
    width: Math.min(parent ? parent.width - 32 : 720, 880)
    height: parent ? parent.height - 40 : 700
    padding: 0
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    background: Rectangle { color: Theme.bg; radius: Theme.radiusCard + 2; border.color: Theme.outline; border.width: 1 }
    Overlay.modal: Rectangle { color: "#aa0b0b12" }

    component ScoreRow: RowLayout {
        property var t
        property bool highlight: false
        Layout.fillWidth: true; spacing: 0
        Helmet { slug: t.helmet; size: 34 }
        Text {
            text: t.name + (t.record ? " (" + t.record + ")" : "")
            color: Theme.ink; font.pixelSize: 14; font.bold: true; wrapMode: Text.WordWrap
            Layout.fillWidth: true; Layout.leftMargin: 8
        }
        Repeater {
            model: dlg.b.quarters || 4
            Text { Layout.preferredWidth: 30; horizontalAlignment: Text.AlignHCenter; text: t.q[index] !== undefined ? t.q[index] : ""; color: Theme.ink; font.pixelSize: 15 }
        }
        Text { Layout.preferredWidth: 44; horizontalAlignment: Text.AlignHCenter; text: t.score; color: Theme.ink; font.pixelSize: 20; font.bold: true }
    }

    contentItem: ColumnLayout {
        spacing: 0
        RowLayout {
            Layout.fillWidth: true; Layout.margins: 14; Layout.bottomMargin: 6
            ScreenTitle { text: "Box score"; font.pixelSize: 24; Layout.fillWidth: true }
            PrimaryButton { text: "Close"; ghost: true; implicitHeight: 34; onClicked: dlg.close() }
        }
        ScrollView {
            id: sv
            Layout.fillWidth: true; Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true
            ColumnLayout {
                width: sv.availableWidth - 28
                x: 14
                spacing: 12

                // scoreboard
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: sc.implicitHeight + 28
                    radius: 14
                    color: Theme.primaryTint
                    border.color: Theme.primaryDark
                    ColumnLayout {
                        id: sc
                        anchors.fill: parent; anchors.margins: 14; spacing: 6
                        Text { text: (dlg.b.final || "") + " · " + (dlg.b.weather || ""); color: Theme.inkMuted; font.pixelSize: 13 }
                        RowLayout {
                            Layout.fillWidth: true; spacing: 0
                            Item { Layout.fillWidth: true }
                            Repeater {
                                model: dlg.b.quarters || 4
                                Text { Layout.preferredWidth: 30; horizontalAlignment: Text.AlignHCenter; text: index < 4 ? index + 1 : "OT"; color: Theme.inkSubtle; font.pixelSize: 13 }
                            }
                            Text { Layout.preferredWidth: 44; horizontalAlignment: Text.AlignHCenter; text: "T"; color: Theme.inkSubtle; font.pixelSize: 13; font.bold: true }
                        }
                        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.outline }
                        Repeater {
                            model: dlg.b.away ? [ { k: "away", t: dlg.b.away }, { k: "home", t: dlg.b.home } ] : []
                            Rectangle {
                                Layout.fillWidth: true
                                implicitHeight: row.implicitHeight + 8
                                radius: 10
                                color: dlg.tab === "Players" && dlg.side === modelData.k ? Qt.alpha(Theme.ink, 0.12) : "transparent"
                                border.color: dlg.tab === "Players" && dlg.side === modelData.k ? Theme.ink : "transparent"
                                ScoreRow { id: row; t: modelData.t; anchors.fill: parent; anchors.margins: 4; anchors.leftMargin: 8; anchors.rightMargin: 8 }
                                TapHandler { enabled: dlg.tab === "Players"; onTapped: dlg.side = modelData.k }
                            }
                        }
                        Text { visible: dlg.tab === "Players"; text: "Tap a team to see its stats."; color: Theme.inkSubtle; font.pixelSize: 12 }
                    }
                }

                RowLayout {
                    spacing: 6
                    Repeater { model: ["Summary", "Teams", "Players", "Drives"]; Chip { text: modelData; selected: dlg.tab === modelData; onClicked: dlg.tab = modelData } }
                }

                // ---- Summary
                ColumnLayout {
                    visible: dlg.tab === "Summary"
                    Layout.fillWidth: true; spacing: 12
                    Card {
                        title: dlg.b.whyTitle || ""
                        fill: dlg.b.tie ? Theme.surfaceAlt : (dlg.b.won ? Theme.goodBg : Theme.warnBg)
                        Repeater {
                            model: dlg.b.why || []
                            RowLayout {
                                Layout.fillWidth: true; spacing: 10
                                Rectangle { width: 22; height: 22; radius: 11; color: Theme.primaryTint; Layout.alignment: Qt.AlignTop
                                    Text { anchors.centerIn: parent; text: index + 1; color: Theme.primary; font.pixelSize: 12; font.bold: true } }
                                Text { text: modelData; color: Theme.ink; font.pixelSize: 14; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                            }
                        }
                    }
                    Card {
                        visible: !!dlg.b.playerOfGame
                        title: "Player of the game"
                        RowLayout {
                            Layout.fillWidth: true; spacing: 12
                            Jersey { visible: !!(dlg.b.playerOfGame && dlg.b.playerOfGame.jersey); number: dlg.b.playerOfGame && dlg.b.playerOfGame.jersey ? dlg.b.playerOfGame.jersey.number : 0; team: dlg.b.playerOfGame && dlg.b.playerOfGame.jersey ? dlg.b.playerOfGame.jersey.teamSlug : ""; lastName: dlg.b.playerOfGame && dlg.b.playerOfGame.jersey ? dlg.b.playerOfGame.jersey.lastName : ""; size: 64 }
                            Helmet { visible: !!dlg.b.playerOfGame && !dlg.b.playerOfGame.jersey; slug: dlg.b.playerOfGame ? dlg.b.playerOfGame.helmet : ""; size: 48 }
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 2
                                Text { text: dlg.b.playerOfGame ? dlg.b.playerOfGame.name + ", " + dlg.b.playerOfGame.pos : ""; color: Theme.ink; font.bold: true; font.pixelSize: 15 }
                                Text { text: dlg.b.playerOfGame ? dlg.b.playerOfGame.summary : ""; color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                            }
                            Text { text: "›"; color: Theme.inkFaint; font.pixelSize: 20 }
                            TapHandler { onTapped: Store.openPlayer(dlg.b.playerOfGame.id) }
                        }
                    }
                    Card {
                        title: "Scoring"
                        Text { visible: (dlg.b.scoring || []).length === 0; text: "Nobody scored. Somehow."; color: Theme.inkMuted }
                        Repeater {
                            model: dlg.b.scoring || []
                            RowLayout {
                                Layout.fillWidth: true; spacing: 8
                                Text { text: modelData.when; color: Theme.inkFaint; font.pixelSize: 12; Layout.preferredWidth: 74 }
                                Text { text: modelData.side; color: Theme.ink; font.bold: true; Layout.preferredWidth: 20 }
                                Text { text: modelData.text; color: Theme.ink; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                                Text { text: modelData.score; color: Theme.inkMuted; font.bold: true; font.pixelSize: 13 }
                            }
                        }
                    }
                    Card {
                        visible: (dlg.b.keyPlays || []).length > 0
                        title: "Key plays"
                        Repeater { model: dlg.b.keyPlays || []; Text { text: modelData; color: Theme.ink; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true } }
                    }
                    Card {
                        visible: (dlg.b.notes || []).length > 0
                        title: "Injuries and fatigue"
                        Repeater { model: dlg.b.notes || []; Text { text: modelData; color: Theme.ink; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true } }
                    }
                }

                // ---- Teams
                Card {
                    visible: dlg.tab === "Teams"
                    RowLayout {
                        Layout.fillWidth: true
                        Text { text: dlg.b.home ? dlg.b.home.name : ""; color: Theme.ink; font.bold: true; Layout.fillWidth: true }
                        Text { text: dlg.b.away ? dlg.b.away.name : ""; color: Theme.ink; font.bold: true; horizontalAlignment: Text.AlignRight }
                    }
                    Repeater {
                        model: dlg.b.teamStats || []
                        ColumnLayout {
                            Layout.fillWidth: true; spacing: 3
                            RowLayout {
                                Layout.fillWidth: true
                                Text { text: modelData.home; color: Theme.ink; font.bold: true; Layout.preferredWidth: 70 }
                                Text { text: modelData.label; color: Theme.inkSubtle; font.pixelSize: 12; horizontalAlignment: Text.AlignHCenter; Layout.fillWidth: true }
                                Text { text: modelData.away; color: Theme.ink; font.bold: true; Layout.preferredWidth: 70; horizontalAlignment: Text.AlignRight }
                            }
                            Rectangle {
                                Layout.fillWidth: true; height: 5; radius: 3; color: Theme.outline
                                Rectangle { width: parent.width * Math.max(0.01, Math.min(0.99, modelData.share)); height: parent.height; radius: 3; color: Theme.primary }
                            }
                        }
                    }
                }

                // ---- Players
                ColumnLayout {
                    visible: dlg.tab === "Players"
                    Layout.fillWidth: true; spacing: 10
                    Flow {
                        Layout.fillWidth: true; spacing: 6
                        Repeater { model: dlg.catKeys; Chip { text: modelData[1]; selected: dlg.pcat === modelData[0]; onClicked: dlg.pcat = modelData[0] } }
                    }
                    Repeater {
                        model: dlg.b.players ? dlg.b.players[dlg.side][dlg.pcat].tables : []
                        Card {
                            title: dlg.b.players[dlg.side][dlg.pcat].tables.length > 1 ? modelData.name : (dlg.b[dlg.side] ? dlg.b[dlg.side].name : "")
                            visible: modelData.rows.length > 0 || dlg.b.players[dlg.side][dlg.pcat].tables.length === 1
                            StatGrid { Layout.fillWidth: true; table: modelData; showTeam: false; empty: "Nothing to show here." }
                        }
                    }
                    Text { text: "Tap a column header to sort. Tap a player to open him."; color: Theme.inkFaint; font.pixelSize: 12 }
                }

                // ---- Drives
                Card {
                    visible: dlg.tab === "Drives"
                    Repeater {
                        model: dlg.b.drives || []
                        RowLayout {
                            Layout.fillWidth: true; spacing: 8
                            Text { text: modelData.side; color: Theme.ink; font.bold: true; Layout.preferredWidth: 26 }
                            Text { text: modelData.when; color: Theme.inkFaint; font.pixelSize: 12; Layout.preferredWidth: 74 }
                            Text { text: modelData.text; color: Theme.inkMuted; font.pixelSize: 13; Layout.fillWidth: true }
                            Text { text: modelData.result; color: Theme.ink; font.bold: true; font.pixelSize: 13 }
                        }
                    }
                }
                Item { Layout.preferredHeight: 8 }
            }
        }
    }
}
