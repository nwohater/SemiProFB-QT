import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// A player's card: ratings, contract, and what you can do about it.
Popup {
    id: dlg
    property int playerId: -1
    property var p: ({})
    function show(id) { playerId = id; refresh(); if (p.name) open(); }  // a player who is gone from the league has no bio to show
    function refresh() { p = JSON.parse(Store.game.playerJson(playerId)); }
    Connections { target: Store.game; function onRevChanged() { if (dlg.opened) dlg.refresh(); } }

    modal: true
    focus: true
    anchors.centerIn: parent
    width: Math.min(parent ? parent.width - 32 : 560, 580)
    height: Math.min(parent ? parent.height - 48 : 700, 720)
    padding: 0
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    background: Rectangle { color: Theme.surface; radius: Theme.radiusCard + 2; border.color: Theme.outline; border.width: 1 }
    Overlay.modal: Rectangle { color: "#aa0b0b12" }

    contentItem: ColumnLayout {
        spacing: 0
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 16
            spacing: 12
            Rectangle {
                width: 56; height: 56; radius: 12; color: Theme.surfaceAlt
                Text { anchors.centerIn: parent; text: dlg.p.ovr !== undefined ? dlg.p.ovr : ""; color: Theme.ovrColor(dlg.p.ovr || 0); font.pixelSize: 26; font.bold: true }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0
                Text { text: dlg.p.name || ""; color: Theme.ink; font.pixelSize: 22; font.bold: true; elide: Text.ElideRight; Layout.fillWidth: true }
                Text { text: (dlg.p.pos || "") + "  ·  age " + (dlg.p.age || "") + "  ·  " + (dlg.p.ht || "") + ", " + (dlg.p.wt || "") + " lb" + (dlg.p.number !== null && dlg.p.number !== undefined ? "  ·  #" + dlg.p.number : "") + (dlg.p.club ? "  ·  " + dlg.p.club : "") + (dlg.p.retired ? "  ·  retired" : ""); color: Theme.inkSubtle; font.pixelSize: 13 }
            }
            Jersey { visible: dlg.p.number !== null && dlg.p.number !== undefined; number: dlg.p.number || 0; team: dlg.p.teamSlug || ""; lastName: dlg.p.lastName || ""; size: 96 }
            PrimaryButton { text: "Close"; ghost: true; implicitHeight: 34; onClicked: dlg.close() }
        }
        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider }

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true
            ColumnLayout {
                width: parent.width - 32
                x: 16
                spacing: 12
                Item { Layout.preferredHeight: 2 }
                Flow {
                    Layout.fillWidth: true
                    spacing: 6
                    Pill { visible: !!dlg.p.local; text: "Hometown hero"; fg: Theme.primary; bg: Theme.primaryTint; strong: true }
                    Pill { visible: !!dlg.p.fan; text: "Fan favorite"; fg: Theme.gold; bg: Theme.warnBg; strong: true }
                    Pill { visible: !!dlg.p.expiring; text: "Contract expiring"; fg: Theme.warnFg; bg: Theme.warnBg; strong: true }
                    Pill { visible: dlg.p.out !== null && dlg.p.out !== undefined; text: "Out " + dlg.p.out + " gm" + (dlg.p.outReason ? " · " + dlg.p.outReason : ""); fg: Theme.badFg; bg: Theme.badBg; strong: true }
                    Pill { visible: (dlg.p.awards || 0) > 0; text: dlg.p.awards + " award" + (dlg.p.awards > 1 ? "s" : ""); fg: Theme.gold; bg: Theme.warnBg }
                    Pill { visible: !!dlg.p.alsoKicks; text: "Also kicks / punts" }
                }
                Card {
                    title: "Ratings"
                    Repeater {
                        model: dlg.p.attrs || []
                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: 30
                            radius: 8
                            color: modelData.key ? Theme.primaryTint : "transparent"
                            border.width: modelData.key ? 1 : 0
                            border.color: Qt.alpha(Theme.primary, 0.35)
                            RowLayout {
                                anchors.fill: parent; anchors.leftMargin: 8; anchors.rightMargin: 8
                                spacing: 8
                                Text { text: modelData.code; color: modelData.key ? Theme.primary : Theme.inkSubtle; font.bold: true; font.pixelSize: 12; Layout.preferredWidth: 26 }
                                Text { text: modelData.name; color: modelData.key ? Theme.ink : Theme.inkSubtle; font.bold: modelData.key; font.pixelSize: 13 }
                                Text { visible: modelData.key; text: "KEY"; color: Theme.primary; font.bold: true; font.pixelSize: 10; font.letterSpacing: 0.8 }
                                Item { Layout.fillWidth: true }
                                Rectangle {
                                    Layout.preferredWidth: 120; height: 8; radius: 4; color: modelData.key ? Theme.bg : Theme.surfaceAlt
                                    Rectangle { width: parent.width * modelData.v / 99; height: parent.height; radius: 4; color: Theme.ovrColor(modelData.v) }
                                }
                                Text { text: modelData.v; color: Theme.ovrColor(modelData.v); font.pixelSize: 13; font.bold: true; Layout.preferredWidth: 28; horizontalAlignment: Text.AlignRight }
                            }
                        }
                    }
                    Text { text: "Highlighted ratings (KEY) are the ones that drive his position's OVR."; color: Theme.inkFaint; font.pixelSize: 11 }
                }
                Card {
                    title: "Background"
                    Text { text: "Hometown: " + (dlg.p.hometown || ""); color: Theme.inkMuted; font.pixelSize: 13 }
                    RowLayout {
                        spacing: 8
                        Text { text: "Day job: " + (dlg.p.job || ""); color: Theme.inkMuted; font.pixelSize: 13 }
                        PrimaryButton { text: "Injury and travel risk"; ghost: true; implicitHeight: 26; leftPadding: 10; rightPadding: 10; font.pixelSize: 12; onClicked: jobDlg.open() }
                    }
                    Text { text: "Experience: " + (dlg.p.yearsLabel || "") + "  ·  " + (dlg.p.acquired || ""); color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                }
                Card {
                    visible: !!dlg.p.contract
                    title: "Contract"
                    Text { text: dlg.p.contract ? dlg.p.contract.ppgText + " per game  ·  " + dlg.p.contract.years + " season(s) left" : ""; color: Theme.ink; font.pixelSize: 15; font.bold: true }
                    Text { text: dlg.p.contract ? "Win bonus " + dlg.p.contract.winBonus + "  ·  commute pay " + dlg.p.contract.commute : ""; color: Theme.inkMuted; font.pixelSize: 13 }
                }
                Card {
                    visible: !!dlg.p.mine && (dlg.p.moves || []).length > 0
                    title: "Change position"
                    Flow {
                        Layout.fillWidth: true; spacing: 6
                        Repeater { model: dlg.p.moves || []; Chip { text: modelData.pos + " " + modelData.ovr; onClicked: { moveDlg.target = modelData; moveDlg.open(); } } }
                    }
                    Text { text: "His ratings stay the same; only the label and depth-chart slot move."; color: Theme.inkFaint; font.pixelSize: 12 }
                }
                Card {
                    visible: !!dlg.p.lastChanges
                    title: "Last offseason"
                    Flow {
                        Layout.fillWidth: true; spacing: 14
                        Repeater {
                            model: dlg.p.lastChanges || []
                            ColumnLayout {
                                spacing: 0
                                readonly property int d: modelData.after - modelData.before
                                Text { text: modelData.code; color: Theme.inkSubtle; font.pixelSize: 12; font.bold: true }
                                Text { text: modelData.before + " → " + modelData.after; color: Theme.ink; font.pixelSize: 13 }
                                Text { text: d > 0 ? "▲ " + d : (d < 0 ? "▼ " + (-d) : "= 0"); color: d > 0 ? Theme.goodFg : (d < 0 ? Theme.badFg : Theme.inkSubtle); font.pixelSize: 12; font.bold: true }
                            }
                        }
                    }
                }
                Card {
                    visible: (dlg.p.titles || []).length > 0 || (dlg.p.awardsWon || []).length > 0
                    title: "Awards"
                    Repeater {
                        model: dlg.p.titles || []
                        RowLayout {
                            Layout.fillWidth: true; spacing: 8
                            Text { text: "🎖"; font.pixelSize: 15 }
                            Text { text: "Season " + modelData.season + " · League champion"; color: Theme.ink; font.bold: true; font.pixelSize: 13; Layout.fillWidth: true }
                            Text { text: modelData.abbr; color: Theme.inkSubtle; font.pixelSize: 12 }
                        }
                    }
                    Repeater {
                        model: dlg.p.awardsWon || []
                        RowLayout {
                            Layout.fillWidth: true; spacing: 8
                            Text { text: "🏆"; font.pixelSize: 15 }
                            Text { text: "Season " + modelData.season + " · " + modelData.title; color: Theme.ink; font.bold: true; font.pixelSize: 13; Layout.fillWidth: true }
                            Text { text: modelData.team; color: Theme.inkSubtle; font.pixelSize: 12 }
                        }
                    }
                }
                Card {
                    title: "Stats"
                    trailing: Row {
                        spacing: 6
                        Chip { text: "Regular"; selected: dlg.statsTab === "regular"; onClicked: dlg.statsTab = "regular" }
                        Chip { text: "Playoffs"; selected: dlg.statsTab === "playoffs"; onClicked: dlg.statsTab = "playoffs" }
                    }
                    Text { visible: !dlg.curStats || dlg.curStats.rows.length === 0; text: "No numbers yet."; color: Theme.inkSubtle; font.pixelSize: 13 }
                    Flickable {
                        visible: !!dlg.curStats && dlg.curStats.rows.length > 0
                        Layout.fillWidth: true
                        Layout.preferredHeight: statTbl.implicitHeight
                        contentWidth: statTbl.implicitWidth; contentHeight: statTbl.implicitHeight
                        flickableDirection: Flickable.HorizontalFlick
                        boundsBehavior: Flickable.StopAtBounds
                        clip: true
                        Column {
                            id: statTbl
                            spacing: 0
                            Row {
                                height: 30
                                Text { width: 48; height: parent.height; text: "Yr"; color: Theme.inkSubtle; font.bold: true; font.pixelSize: 12; verticalAlignment: Text.AlignVCenter }
                                Text { width: 44; height: parent.height; text: "Team"; color: Theme.inkSubtle; font.bold: true; font.pixelSize: 12; verticalAlignment: Text.AlignVCenter }
                                Repeater { model: dlg.curStats ? dlg.curStats.heads : []; Text { width: 62; height: 30; text: modelData; color: Theme.inkSubtle; font.bold: true; font.pixelSize: 12; horizontalAlignment: Text.AlignRight; verticalAlignment: Text.AlignVCenter; rightPadding: 6 } }
                            }
                            Repeater {
                                model: dlg.curStats ? dlg.curStats.rows : []
                                Row {
                                    id: statRow
                                    readonly property bool isTotal: modelData.total
                                    height: 30
                                    Text { width: 48; height: 30; text: modelData.yr; color: Theme.ink; font.bold: statRow.isTotal; font.pixelSize: 13; verticalAlignment: Text.AlignVCenter }
                                    Text { width: 44; height: 30; text: modelData.team; color: Theme.inkMuted; font.pixelSize: 13; verticalAlignment: Text.AlignVCenter }
                                    Repeater { model: modelData.cells; Text { width: 62; height: 30; text: modelData; color: Theme.ink; font.bold: statRow.isTotal; font.pixelSize: 13; horizontalAlignment: Text.AlignRight; verticalAlignment: Text.AlignVCenter; rightPadding: 6 } }
                                }
                            }
                        }
                    }
                }
                Card {
                    title: "Injury & absence log"
                    trailing: Text { visible: !!dlg.p.absences && !dlg.p.absences.empty; text: dlg.p.absences ? dlg.p.absences.total + " missed" : ""; color: Theme.inkSubtle; font.pixelSize: 12; font.bold: true }
                    Text { visible: !!dlg.p.absences && dlg.p.absences.empty; text: "Nothing yet. Not a scratch, not a late shift."; color: Theme.inkSubtle; font.pixelSize: 13 }
                    Repeater {
                        model: dlg.p.absences ? dlg.p.absences.seasons : []
                        ColumnLayout {
                            Layout.fillWidth: true; spacing: 4
                            Text { text: "SEASON " + modelData.season + " · " + modelData.missed + " games missed"; color: Theme.inkSubtle; font.pixelSize: 11; font.bold: true; font.letterSpacing: 0.6; topPadding: 6 }
                            Repeater {
                                model: modelData.rows
                                RowLayout {
                                    Layout.fillWidth: true; spacing: 10
                                    Rectangle {
                                        Layout.preferredWidth: 70; Layout.preferredHeight: 24; radius: 6
                                        color: modelData.kind === "WORK" ? "#b9690b" : (modelData.kind === "CONDUCT" ? "#a23a3a" : "#2c6fa8")
                                        Text { anchors.centerIn: parent; text: modelData.kind; color: "white"; font.pixelSize: 11; font.bold: true; font.letterSpacing: 0.8 }
                                    }
                                    ColumnLayout {
                                        Layout.fillWidth: true; spacing: 0
                                        Text { text: modelData.what; color: Theme.ink; font.bold: true; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                                        Text { text: modelData.when + " · " + modelData.status; color: Theme.inkSubtle; font.pixelSize: 12 }
                                    }
                                }
                            }
                        }
                    }
                }
                Item { Layout.preferredHeight: 4 }
            }
        }

        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider }
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 12
            spacing: 8
            PrimaryButton {
                visible: !!dlg.p.mine
                text: dlg.p.active ? "Move to inactive" : "Dress for games"
                ghost: true
                onClicked: Store.game.setActive(dlg.playerId, !dlg.p.active)
            }
            PrimaryButton {
                visible: !!dlg.p.mine && !!dlg.p.expiring
                text: "Re-sign"
                onClicked: { dlg.close(); Store.openNegotiation(dlg.playerId); }
            }
            PrimaryButton {
                visible: !!dlg.p.freeAgent
                text: "Make an offer"
                onClicked: { dlg.close(); Store.openNegotiation(dlg.playerId); }
            }
            Item { Layout.fillWidth: true }
            PrimaryButton {
                visible: !!dlg.p.mine
                text: dlg.p.releaseCost > 0 ? "Release (" + Theme.money(dlg.p.releaseCost) + ")" : "Release"
                danger: true
                onClicked: confirm.open()
            }
        }
    }

    property string statsTab: "regular"
    readonly property var curStats: p.stats ? p.stats[statsTab] : null

    Popup {
        id: jobDlg
        parent: Overlay.overlay
        anchors.centerIn: parent
        modal: true
        width: Math.min(parent ? parent.width - 32 : 420, 420)
        padding: 18
        background: Rectangle { color: Theme.surface; radius: Theme.radiusCard + 2; border.color: Theme.outline }
        Overlay.modal: Rectangle { color: "#aa0b0b12" }
        readonly property var j: dlg.p.jobInfo || ({})
        contentItem: ColumnLayout {
            spacing: 10
            Text { text: jobDlg.j.name || ""; color: Theme.ink; font.pixelSize: 20; font.bold: true }
            Text { text: (dlg.p.name || "") + "'s day job"; color: Theme.inkSubtle; font.pixelSize: 12 }
            Repeater {
                model: [ { label: "Injury risk", v: jobDlg.j.injury, band: jobDlg.j.injuryBand, hint: "Chance he gets hurt on the job and misses games.", pct: jobDlg.j.injuryPct },
                         { label: "Travel risk", v: jobDlg.j.travel, band: jobDlg.j.travelBand, hint: "Chance work travel keeps him from the next game.", pct: jobDlg.j.travelPct } ]
                ColumnLayout {
                    Layout.fillWidth: true; spacing: 4
                    RowLayout { Layout.fillWidth: true; Text { text: modelData.label; color: Theme.ink; font.bold: true; Layout.fillWidth: true } Text { text: modelData.band + " · " + modelData.v; color: Theme.inkMuted; font.bold: true } }
                    Rectangle { Layout.fillWidth: true; height: 8; radius: 4; color: Theme.divider
                        Rectangle { width: parent.width * Math.min(1, (modelData.v || 0) / 99); height: parent.height; radius: 4; color: modelData.v >= 50 ? Theme.badFg : (modelData.v >= 25 ? Theme.warnFg : Theme.primary) } }
                    Text { text: modelData.hint + " About " + modelData.pct + "."; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                }
            }
            PrimaryButton { text: "Close"; ghost: true; Layout.alignment: Qt.AlignRight; implicitHeight: 36; onClicked: jobDlg.close() }
        }
    }
    ConfirmDialog {
        id: moveDlg
        property var target: ({ pos: "", ovr: 0 })
        title: "Move " + (dlg.p.name || "") + " to " + target.pos + "?"
        body: (dlg.p.pos || "") + " " + (dlg.p.ovr || 0) + " → " + target.pos + " " + target.ovr + ". He comes off your " + (dlg.p.pos || "") + " depth chart and can be placed at " + target.pos + " there."
        action: "Move"
        onConfirmed: Store.game.changePosition(dlg.playerId, target.pos)
    }

    Dialog {
        id: confirm
        parent: Overlay.overlay
        anchors.centerIn: parent
        modal: true
        title: "Release " + (dlg.p.name || "") + "?"
        standardButtons: Dialog.Cancel | Dialog.Ok
        onAccepted: { Store.game.releasePlayer(dlg.playerId); dlg.close(); }
        contentItem: Text { text: dlg.p.releaseCost > 0 ? "Cutting him now costs " + Theme.money(dlg.p.releaseCost) + " in remaining contract money." : "He leaves at no cost."; color: Theme.inkMuted; wrapMode: Text.WordWrap; width: 280 }
        background: Rectangle { color: Theme.surface; radius: Theme.radiusCard; border.color: Theme.outline }
        header: Text { text: confirm.title; color: Theme.ink; font.bold: true; font.pixelSize: 16; padding: 16 }
    }
}
