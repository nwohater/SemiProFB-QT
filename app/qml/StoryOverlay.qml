import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Full-window season-end moments: the champion reveal, the retirements list, and game over.
Popup {
    id: dlg
    property string kind: ""
    readonly property var vm: { Store.game.rev; return kind === "" ? ({}) : JSON.parse(Store.game.storyJson(kind)); }
    property real t: 0
    function show(k) {
        kind = k;
        t = 0;
        open();
        if (k === "champion") anim.restart();
    }
    function ramp(from, to) { return Math.max(0, Math.min(1, (t - from) / (to - from))); }

    modal: true
    focus: true
    parent: Overlay.overlay
    x: 0; y: 0
    width: parent ? parent.width : 800
    height: parent ? parent.height : 600
    padding: 0
    closePolicy: Popup.CloseOnEscape
    background: Rectangle { color: dlg.kind === "champion" ? Theme.primaryDark : Theme.bg }
    Overlay.modal: Item {}

    component RetRow: RowLayout {
        property var e
        Layout.fillWidth: true; spacing: 12
        Helmet { slug: e.helmet; size: 44 }
        ColumnLayout {
            Layout.fillWidth: true; spacing: 1
            RowLayout {
                spacing: 6
                Text { text: e.name; color: Theme.ink; font.bold: true; font.pixelSize: 14 }
                Text { visible: e.hof; text: "🏆"; font.pixelSize: 14 }
            }
            Text { text: e.line; color: Theme.inkSubtle; font.pixelSize: 12 }
        }
        Text { text: "›"; color: Theme.inkFaint; font.pixelSize: 18 }
        TapHandler { onTapped: Store.openPlayer(e.id) }
    }

    NumberAnimation { id: anim; target: dlg; property: "t"; from: 0; to: 1; duration: 3200; easing.type: Easing.Linear }

    contentItem: Item {
        // ---------------------------------------------------------------- champion
        ColumnLayout {
            visible: dlg.kind === "champion"
            anchors.fill: parent
            anchors.margins: 16
            spacing: 12
            Item {
                Layout.fillWidth: true; Layout.fillHeight: true
                ColumnLayout {
                    anchors.centerIn: parent
                    width: Math.min(parent.width, 760)
                    spacing: 14
                    Helmet {
                        slug: dlg.vm.helmet || ""; size: 140
                        Layout.alignment: Qt.AlignHCenter
                        opacity: dlg.ramp(0, 0.1)
                        scale: 0.4 + 0.6 * Math.min(1, dlg.ramp(0, 0.2) * 1.1)
                    }
                    Text {
                        text: dlg.vm.name || ""
                        Layout.alignment: Qt.AlignHCenter
                        color: dlg.vm.iWon ? Theme.gold : "white"
                        font.pixelSize: 40; font.bold: true
                        opacity: dlg.ramp(0.05, 0.2)
                    }
                    Item {
                        Layout.fillWidth: true
                        Layout.preferredHeight: width * 675 / 1200
                        clip: true
                        Item {
                            id: photo
                            width: parent.width; height: parent.height
                            x: (1 - dlg.ramp(0.2, 0.55) * dlg.ramp(0.2, 0.55) * (3 - 2 * dlg.ramp(0.2, 0.55))) * width * 1.1
                            Image {
                                anchors.fill: parent
                                source: Store.game.assetsUrl ? Store.game.assetsUrl + "artwork/stories/team_photo.webp" : ""
                                fillMode: Image.PreserveAspectCrop; asynchronous: true
                            }
                            Rectangle {
                                anchors.horizontalCenter: parent.horizontalCenter
                                anchors.bottom: parent.bottom; anchors.bottomMargin: parent.width * 0.0095
                                implicitWidth: stamp.implicitWidth + parent.width * 0.06
                                implicitHeight: stamp.implicitHeight + parent.width * 0.008
                                width: implicitWidth; height: implicitHeight
                                color: "transparent"
                                border.color: Qt.alpha(Theme.gold, 0.9); border.width: Math.max(2, parent.width * 0.005); radius: parent.width * 0.01
                                rotation: -2
                                opacity: dlg.ramp(0.65, 0.8)
                                scale: 1.9 - 0.9 * dlg.ramp(0.65, 0.8)
                                Text {
                                    id: stamp
                                    anchors.centerIn: parent
                                    text: "SEASON " + dlg.vm.season + " CHAMPS"
                                    color: Qt.alpha(Theme.gold, 0.95)
                                    font.pixelSize: photo.width * 0.045; font.weight: Font.Black; font.letterSpacing: photo.width * 0.004
                                }
                            }
                        }
                    }
                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: dlg.vm.iWon ? "The town is losing its mind." : "are your league champions."
                        color: Theme.inkMuted; font.pixelSize: 16
                        opacity: dlg.ramp(0.8, 1)
                    }
                }
            }
            PrimaryButton { text: "Continue"; Layout.alignment: Qt.AlignHCenter; Layout.preferredWidth: 320; implicitHeight: 52; onClicked: dlg.close() }
        }

        // ---------------------------------------------------------------- retirements
        ColumnLayout {
            visible: dlg.kind === "retire"
            anchors.fill: parent
            spacing: 0
            ScrollView {
                id: rsv
                Layout.fillWidth: true; Layout.fillHeight: true
                contentWidth: availableWidth
                clip: true
                ColumnLayout {
                    width: Math.min(rsv.availableWidth - 2 * Theme.gutter, 900)
                    x: Math.max(Theme.gutter, (rsv.availableWidth - width) / 2)
                    y: Theme.gutter
                    spacing: 12
                    ScreenTitle { text: "Retirements" }
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: Math.min(width * 9 / 16, 280)
                        radius: Theme.radiusCard; color: Theme.surface; clip: true
                        Image { anchors.fill: parent; source: Store.game.assetsUrl ? Store.game.assetsUrl + "artwork/stories/retirement.webp" : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true }
                    }
                    ColumnLayout {
                        Layout.alignment: Qt.AlignHCenter; spacing: 2
                        Text { text: dlg.vm.count !== undefined ? dlg.vm.count : ""; color: Theme.primary; font.pixelSize: 44; font.bold: true; Layout.alignment: Qt.AlignHCenter }
                        Text { text: "PLAYER" + (dlg.vm.count === 1 ? "" : "S") + " RETIRED"; color: Theme.ink; font.bold: true; font.letterSpacing: 1.2; Layout.alignment: Qt.AlignHCenter }
                        Text { text: "after season " + dlg.vm.season; color: Theme.inkMuted; Layout.alignment: Qt.AlignHCenter }
                        Pill {
                            Layout.alignment: Qt.AlignHCenter; Layout.topMargin: 6
                            text: (dlg.vm.mine || []).length === 0 ? "None from your team" : dlg.vm.mine.length + " from your team"
                            fg: Theme.primary; bg: Theme.primaryTint; strong: true
                        }
                    }
                    Card {
                        title: "Your team"
                        Text { visible: (dlg.vm.mine || []).length === 0; text: "Nobody on your roster retired this year."; color: Theme.inkMuted; font.pixelSize: 13 }
                        Repeater { model: dlg.vm.mine || []; RetRow { e: modelData } }
                    }
                    Card {
                        title: "Around the league"
                        Text { visible: (dlg.vm.others || []).length === 0; text: "Nobody else retired."; color: Theme.inkMuted; font.pixelSize: 13 }
                        Repeater { model: dlg.vm.others || []; RetRow { e: modelData } }
                    }
                    Item { Layout.preferredHeight: 8 }
                }
            }
            Rectangle {
                Layout.fillWidth: true; Layout.preferredHeight: 68; color: Theme.bg
                Rectangle { width: parent.width; height: 1; color: Theme.divider }
                PrimaryButton { anchors.centerIn: parent; width: Math.min(parent.width - 32, 420); implicitHeight: 46; text: "On to the offseason"; onClicked: dlg.close() }
            }
        }

        // ---------------------------------------------------------------- game over
        ScrollView {
            id: gsv
            visible: dlg.kind === "gameover"
            anchors.fill: parent
            contentWidth: availableWidth
            clip: true
            ColumnLayout {
                width: Math.min(gsv.availableWidth - 2 * Theme.gutter, 760)
                x: Math.max(Theme.gutter, (gsv.availableWidth - width) / 2)
                y: Theme.gutter
                spacing: 12
                ScreenTitle { text: "Game over" }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: width * 9 / 16
                    radius: Theme.radiusCard; color: Theme.surface; clip: true
                    Image { anchors.fill: parent; source: Store.game.assetsUrl ? Store.game.assetsUrl + "artwork/stories/broke.webp" : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true }
                }
                Card {
                    Text { text: (dlg.vm.name || "") + " have gone out of business."; color: Theme.ink; font.pixelSize: 20; font.bold: true; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    Text {
                        text: "The bank has changed the locks. The mascot has been seen hitchhiking out of " + (dlg.vm.town || "") + ". Local radio played a moment of silence, then a car commercial."
                        color: Theme.inkMuted; font.pixelSize: 14; wrapMode: Text.WordWrap; Layout.fillWidth: true
                    }
                }
                Card {
                    title: "Your run"
                    RowLayout {
                        Layout.fillWidth: true; spacing: 12
                        StatTile { label: "Seasons played"; value: "" + (dlg.vm.seasons !== undefined ? dlg.vm.seasons : ""); Layout.fillWidth: true }
                        StatTile { label: "Titles"; value: "" + (dlg.vm.titles !== undefined ? dlg.vm.titles : ""); Layout.fillWidth: true }
                        StatTile { label: "Overall record"; value: dlg.vm.career || ""; Layout.fillWidth: true }
                    }
                }
                Text { text: "Final season: " + (dlg.vm.finalSeason || ""); color: Theme.inkSubtle; font.pixelSize: 13 }
                PrimaryButton {
                    Layout.fillWidth: true; implicitHeight: 50
                    text: "Start a new franchise"
                    onClicked: { dlg.close(); Store.game.quitToTitle(); }
                }
                Item { Layout.preferredHeight: 8 }
            }
        }
    }
}
