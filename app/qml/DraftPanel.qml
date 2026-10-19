import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The Premium Player Draft: prospects board, pick order and picks so far.
Item {
    id: root
    property string view: "Prospects"
    property string pos: "All"
    property string sort: "OVR"
    property bool localOnly: false
    property var pending: null     // prospect awaiting a draft confirmation
    property string error: ""
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.draftJson()); }
    readonly property var groupMap: ({ "RB": ["RB", "FB"], "OL": ["OT", "OG", "C"], "DL": ["DE", "DT"], "LB": ["MLB", "OLB"], "DB": ["CB", "S"] })
    readonly property var rows: {
        var rs = (vm.prospects || []).filter(function(p) {
            if (pos !== "All" && (groupMap[pos] || [pos]).indexOf(p.pos) < 0) return false;
            return !localOnly || p.local;
        });
        rs.sort(function(a, b) {
            if (sort === "Pay/game") return a.pay - b.pay;
            if (sort === "Commute") return a.commute - b.commute || b.ovr - a.ovr;
            if (sort === "Age") return a.age - b.age;
            if (sort === "Name") return a.name.localeCompare(b.name);
            return b.ovr - a.ovr;
        });
        return rs.slice(0, 80);
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 8
        Text { visible: !!root.vm.none; text: "There is no draft right now."; color: Theme.inkMuted }

        Card {
            visible: !root.vm.none
            title: "Premium Player Draft"
            trailing: Text { text: (root.vm.made || 0) + " of " + (root.vm.total || 0); color: Theme.inkSubtle; font.pixelSize: 12 }
            RowLayout {
                visible: !root.vm.complete
                spacing: 12
                Helmet { slug: root.vm.clockTeam ? root.vm.clockTeam.helmet : ""; size: 52 }
                ColumnLayout {
                    Layout.fillWidth: true; spacing: 0
                    Text { text: root.vm.onClock ? "YOU ARE ON THE CLOCK" : "ON THE CLOCK"; color: root.vm.onClock ? Theme.primary : Theme.inkSubtle; font.pixelSize: 11; font.bold: true; font.letterSpacing: 1 }
                    Text { text: root.vm.onClock ? "Round " + root.vm.round + ", pick #" + root.vm.overall : (root.vm.clockTeam ? root.vm.clockTeam.name : ""); color: Theme.ink; font.pixelSize: 17; font.bold: true }
                    Text { text: (root.vm.onClock ? "" : "Round " + root.vm.round + ", pick #" + root.vm.overall + " · ") + root.vm.slotSeasons + "-season deal, " + (root.vm.slotPpgText || "") + " per game"; color: Theme.inkSubtle; font.pixelSize: 12 }
                }
            }
            Text { visible: !!root.vm.complete; text: "The draft is over: " + ((root.vm.made || 0) - (root.vm.passed || 0)) + " players drafted, " + (root.vm.passed || 0) + " picks passed."; color: Theme.ink; font.bold: true }
            Text { text: (root.vm.mine || []).length === 0 ? "You have no picks left." : "Your picks: " + root.vm.mine.slice(0, 6).map(function(n) { return "#" + n; }).join(", ") + (root.vm.mine.length > 6 ? " and " + (root.vm.mine.length - 6) + " more" : ""); color: Theme.ink; font.pixelSize: 13; font.bold: true }
            Text { text: "Your board: " + (root.vm.strategy || "") + ((root.vm.focus || []).length ? " · needs " + root.vm.focus.join(", ") : ""); color: Theme.inkSubtle; font.pixelSize: 12 }
            Text { visible: !!root.vm.broke && !root.vm.complete; text: "Your club " + root.vm.broke + ", so you have to pass on your picks."; color: Theme.badFg; font.pixelSize: 13; font.bold: true; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            Text { visible: root.error !== ""; text: root.error; color: Theme.badFg; font.pixelSize: 12 }
            Flow {
                visible: !root.vm.complete
                Layout.fillWidth: true; spacing: 8
                PrimaryButton { visible: !!root.vm.onClock; text: root.vm.broke ? "Pass (forced)" : "Pass on this pick"; ghost: true; implicitHeight: 36; onClicked: passDlg.open() }
                PrimaryButton { visible: !root.vm.onClock; text: (root.vm.mine || []).length === 0 ? "Finish the draft" : "Sim to my pick"; implicitHeight: 36; onClicked: Store.game.simDraftToUser() }
                PrimaryButton { text: "Auto-draft the rest"; ghost: true; implicitHeight: 36; onClicked: autoDlg.open() }
            }
        }

        RowLayout {
            visible: !root.vm.none
            spacing: 6
            Chip { text: "Prospects"; selected: root.view === "Prospects"; onClicked: root.view = "Prospects" }
            Chip { text: "Picks"; selected: root.view === "Picks"; onClicked: root.view = "Picks" }
            Item { Layout.fillWidth: true }
        }

        // ---------------- prospects
        ColumnLayout {
            visible: !root.vm.none && root.view === "Prospects"
            Layout.fillWidth: true; Layout.fillHeight: true
            spacing: 6
            Flow {
                Layout.fillWidth: true; spacing: 5
                Repeater { model: ["All", "QB", "RB", "WR", "TE", "OL", "DL", "LB", "DB", "K", "P"]; Chip { text: modelData; selected: root.pos === modelData; implicitHeight: 28; font.pixelSize: 12; onClicked: root.pos = modelData } }
            }
            Flow {
                Layout.fillWidth: true; spacing: 5
                Text { text: "Sort"; color: Theme.inkSubtle; font.pixelSize: 12; height: 28; verticalAlignment: Text.AlignVCenter }
                Repeater { model: ["OVR", "Pay/game", "Commute", "Age", "Name"]; Chip { text: modelData; selected: root.sort === modelData; implicitHeight: 28; font.pixelSize: 12; onClicked: root.sort = modelData } }
                Chip { text: "Local only"; selected: root.localOnly; implicitHeight: 28; font.pixelSize: 12; onClicked: root.localOnly = !root.localOnly }
            }
            Text { text: (root.vm.prospects || []).length === 0 ? "Nobody is left on the board. Undrafted players are free agents now." : root.rows.length + " prospects. Pay is what he would cost per game with the pick on the clock (" + (root.vm.slotPpgText || "") + " plus commute)."; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            ListView {
                Layout.fillWidth: true; Layout.fillHeight: true
                clip: true; spacing: 4
                model: root.rows
                ScrollBar.vertical: ScrollBar {}
                delegate: Rectangle {
                    width: ListView.view.width - 8; height: 62
                    radius: Theme.radiusChip + 2
                    color: hv3.hovered ? Theme.surfaceAlt : Theme.surface; border.color: Theme.divider
                    HoverHandler { id: hv3; cursorShape: Qt.PointingHandCursor }
                    TapHandler { onTapped: Store.openPlayer(modelData.id) }
                    RowLayout {
                        anchors.fill: parent; anchors.margins: 8; spacing: 10
                        Rectangle { width: 40; height: 40; radius: 10; color: Theme.surfaceAlt; Text { anchors.centerIn: parent; text: modelData.ovr; color: Theme.ovrColor(modelData.ovr); font.bold: true; font.pixelSize: 17 } }
                        ColumnLayout {
                            Layout.fillWidth: true; spacing: 0
                            RowLayout { spacing: 4
                                Text { text: modelData.name; color: Theme.ink; font.bold: true; font.pixelSize: 14; elide: Text.ElideRight; Layout.maximumWidth: 220 }
                                Text { visible: modelData.top3; text: "★"; color: modelData.best ? Theme.gold : Theme.inkFaint; font.pixelSize: 13 }
                            }
                            Text { text: modelData.pos + " · " + modelData.age + " · " + modelData.size + " · " + modelData.hometown + (modelData.local ? " · local" : ""); color: Theme.inkSubtle; font.pixelSize: 12; elide: Text.ElideRight; Layout.fillWidth: true }
                        }
                        ColumnLayout {
                            spacing: 0
                            Text { text: modelData.payText + "/g"; color: Theme.ink; font.bold: true; font.pixelSize: 13; Layout.alignment: Qt.AlignRight }
                            Text { text: modelData.local ? "no commute" : modelData.commuteText + " commute"; color: Theme.inkSubtle; font.pixelSize: 11; Layout.alignment: Qt.AlignRight }
                        }
                        PrimaryButton { visible: !!root.vm.canDraft; text: "Draft"; implicitHeight: 32; font.pixelSize: 13; onClicked: { root.pending = modelData; draftDlg.open(); } }
                    }
                }
            }
        }

        // ---------------- picks
        ScrollView {
            id: pickSv
            visible: !root.vm.none && root.view === "Picks"
            Layout.fillWidth: true; Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true
            ColumnLayout {
                width: pickSv.availableWidth - 8
                spacing: 3
                Text { visible: (root.vm.picks || []).length === 0; text: "Pick order — worst finish first, champion last. The same order repeats for every round."; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                Repeater {
                    model: (root.vm.picks || []).length === 0 ? (root.vm.order || []) : []
                    RowLayout {
                        Layout.fillWidth: true; spacing: 8
                        Text { text: modelData.pick; color: Theme.inkFaint; Layout.preferredWidth: 26 }
                        Helmet { slug: modelData.team.helmet; size: 26 }
                        Text { text: modelData.team.name + (modelData.from ? " (from " + modelData.from + ")" : ""); color: modelData.mine ? Theme.primary : Theme.ink; font.bold: modelData.mine; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { text: modelData.record; color: Theme.inkSubtle; font.pixelSize: 12 }
                    }
                }
                Repeater {
                    model: (root.vm.picks || []).slice().reverse()
                    Rectangle {
                        Layout.fillWidth: true; implicitHeight: 48; radius: Theme.radiusChip
                        color: modelData.mine ? Theme.primaryTint : "transparent"
                        TapHandler { enabled: modelData.playerId !== null; onTapped: Store.openPlayer(modelData.playerId) }
                        RowLayout {
                            anchors.fill: parent; anchors.leftMargin: 6; anchors.rightMargin: 6; spacing: 8
                            Text { text: "R" + modelData.round + " #" + modelData.overall; color: Theme.inkSubtle; font.pixelSize: 12; font.bold: true; Layout.preferredWidth: 58 }
                            Helmet { slug: modelData.team.helmet; size: 28 }
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 0
                                Text { text: modelData.forfeited ? modelData.team.town + " passed" : modelData.name; color: Theme.ink; font.bold: !modelData.forfeited; font.italic: modelData.forfeited; font.pixelSize: 13; elide: Text.ElideRight; Layout.fillWidth: true }
                                Text { text: modelData.forfeited ? (modelData.reason || "") : modelData.pos + " · " + modelData.age + " · " + modelData.team.town; color: Theme.inkSubtle; font.pixelSize: 11; elide: Text.ElideRight; Layout.fillWidth: true }
                            }
                            Text { visible: !modelData.forfeited; text: modelData.ovr; color: Theme.ovrColor(modelData.ovr); font.bold: true; font.pixelSize: 15 }
                            Text { visible: !modelData.forfeited; text: modelData.payText + (modelData.commuteText !== "" ? "+" + modelData.commuteText : ""); color: Theme.inkSubtle; font.pixelSize: 11 }
                        }
                    }
                }
            }
        }
    }

    Dialog {
        id: draftDlg
        parent: Overlay.overlay; anchors.centerIn: parent; modal: true
        title: root.pending ? "Draft " + root.pending.name + "?" : ""
        standardButtons: Dialog.Cancel | Dialog.Ok
        onAccepted: { root.error = Store.game.draftPlayer(root.pending.id); }
        contentItem: Text { text: root.pending ? root.pending.pos + " · OVR " + root.pending.ovr + " · age " + root.pending.age + "\n\nRound " + root.vm.round + ", pick #" + root.vm.overall + ": a fixed " + root.vm.slotSeasons + "-season rookie deal at " + root.vm.slotPpgText + " per game" + (root.pending.local ? " (local, no commute)." : " plus " + root.pending.commuteText + " commute.") : ""; color: Theme.inkMuted; wrapMode: Text.WordWrap; width: 320 }
        background: Rectangle { color: Theme.surface; radius: Theme.radiusCard; border.color: Theme.outline }
        header: Text { text: draftDlg.title; color: Theme.ink; font.bold: true; font.pixelSize: 16; padding: 16 }
    }
    Dialog {
        id: passDlg
        parent: Overlay.overlay; anchors.centerIn: parent; modal: true
        title: "Pass on this pick?"
        standardButtons: Dialog.Cancel | Dialog.Ok
        onAccepted: root.error = Store.game.passPick()
        contentItem: Text { text: root.vm.broke ? "Your club " + root.vm.broke + ", so the pick is passed." : "The pick is gone for good and the next team goes on the clock. You keep your cash, but you do not get a player."; color: Theme.inkMuted; wrapMode: Text.WordWrap; width: 320 }
        background: Rectangle { color: Theme.surface; radius: Theme.radiusCard; border.color: Theme.outline }
        header: Text { text: passDlg.title; color: Theme.ink; font.bold: true; font.pixelSize: 16; padding: 16 }
    }
    Dialog {
        id: autoDlg
        parent: Overlay.overlay; anchors.centerIn: parent; modal: true
        title: "Auto-draft the rest?"
        standardButtons: Dialog.Cancel | Dialog.Ok
        onAccepted: Store.game.simDraftAll()
        contentItem: Text { text: "Every remaining pick is made for you, including yours: the best player on your board each time (or a pass if you are broke)."; color: Theme.inkMuted; wrapMode: Text.WordWrap; width: 320 }
        background: Rectangle { color: Theme.surface; radius: Theme.radiusCard; border.color: Theme.outline }
        header: Text { text: autoDlg.title; color: Theme.ink; font.bold: true; font.pixelSize: 16; padding: 16 }
    }
}
