import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Offer a deal: pay per game, length, optional signing bonus. Up to two rounds; they answer in character.
Popup {
    id: dlg
    property int playerId: -1
    property var info: ({})
    property int seasons: 3
    property int offerPpg: 0
    property int bonus: 0
    property int round: 1
    property var last: null
    property bool touchedPpg: false

    function show(id) {
        playerId = id; seasons = 3; bonus = 0; round = 1; last = null; touchedPpg = false;
        reload(); open();
    }
    function reload() {
        info = JSON.parse(Store.game.negotiationJson(playerId, seasons));
        if (!touchedPpg) offerPpg = info.want || 0;
    }

    modal: true
    focus: true
    anchors.centerIn: parent
    width: Math.min(parent ? parent.width - 32 : 460, 460)
    padding: 18
    closePolicy: Popup.CloseOnEscape
    background: Rectangle { color: Theme.surface; radius: Theme.radiusCard + 2; border.color: Theme.outline; border.width: 1 }
    Overlay.modal: Rectangle { color: "#aa0b0b12" }

    contentItem: ColumnLayout {
        spacing: 10
        Text { text: (dlg.info.name || "") + "  ·  " + (dlg.info.pos || "") + "  ·  OVR " + (dlg.info.ovr || ""); color: Theme.ink; font.pixelSize: 18; font.bold: true; Layout.fillWidth: true; elide: Text.ElideRight }
        Text { text: "Round " + dlg.round + " of " + (dlg.info.maxRounds || 2) + ". Shorter deals cost more per game."; color: Theme.inkSubtle; font.pixelSize: 12 }
        Text { visible: !!dlg.info.awardNote; text: dlg.info.awardNote || ""; color: Theme.gold; font.pixelSize: 12; font.bold: true; wrapMode: Text.WordWrap; Layout.fillWidth: true }
        Repeater { model: dlg.info.notes || []; Text { text: modelData; color: Theme.warnFg; font.pixelSize: 12; font.bold: true; wrapMode: Text.WordWrap; Layout.fillWidth: true } }

        RowLayout {
            spacing: 6
            Repeater {
                model: [1, 2, 3]
                Chip { text: modelData + " season" + (modelData > 1 ? "s" : ""); selected: dlg.seasons === modelData; onClicked: { dlg.seasons = modelData; dlg.touchedPpg = false; dlg.reload(); } }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            PrimaryButton { text: "−"; ghost: true; implicitHeight: 38; implicitWidth: 44; leftPadding: 4; rightPadding: 4; onClicked: { dlg.touchedPpg = true; dlg.offerPpg = Math.max(10, dlg.offerPpg - 5); } }
            Text { text: Theme.money(dlg.offerPpg) + " / game"; color: Theme.ink; font.pixelSize: 22; font.bold: true; horizontalAlignment: Text.AlignHCenter; Layout.fillWidth: true }
            PrimaryButton { text: "+"; ghost: true; implicitHeight: 38; implicitWidth: 44; leftPadding: 4; rightPadding: 4; onClicked: { dlg.touchedPpg = true; dlg.offerPpg += 5; } }
        }
        Text { text: "Roughly " + (dlg.info.wantText || "") + " gets a yes. Signing bonus " + Theme.money(dlg.bonus) + " up front."; color: Theme.inkSubtle; font.pixelSize: 12 }
        Slide { Layout.fillWidth: true; from: 0; to: 2000; stepSize: 100; value: dlg.bonus; onMoved: dlg.bonus = Math.round(value) }

        Rectangle {
            visible: !!dlg.last
            Layout.fillWidth: true
            implicitHeight: quote.implicitHeight + 20
            radius: Theme.radiusChip
            color: Theme.surfaceAlt
            Text { id: quote; anchors.fill: parent; anchors.margins: 10; text: dlg.last ? "“" + dlg.last.line + "”" : ""; color: Theme.inkMuted; font.italic: true; font.pixelSize: 13; wrapMode: Text.WordWrap }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            PrimaryButton {
                text: "Make offer"
                Layout.fillWidth: true
                onClicked: {
                    var r = JSON.parse(Store.game.offer(dlg.playerId, dlg.offerPpg, dlg.seasons, dlg.bonus, dlg.round));
                    if (r.signed) { dlg.close(); return; }
                    dlg.last = r;
                    if (r.outcome === "counter" && r.counter) { dlg.touchedPpg = true; dlg.offerPpg = r.counter; }
                    if (dlg.round < (dlg.info.maxRounds || 2)) dlg.round++;
                }
            }
            PrimaryButton { text: "Walk away"; ghost: true; onClicked: dlg.close() }
        }
    }
}
