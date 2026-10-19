import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Sound, your franchise and its saves, difficulty, tutorial, about. Sound and About also work before a franchise exists.
Item {
    id: root
    signal replayTutorial()
    signal abandoned()
    property int tick: 0     // bumps to re-read the save info after a backup
    readonly property var vm: { Store.game.rev; tick; return JSON.parse(Store.game.settingsJson()); }
    readonly property var fr: vm.franchise || null
    readonly property var town: fr ? fr.town : null

    function bytes(b) { return b < 1024 ? b + " B" : (b < 1048576 ? Math.round(b / 1024) + " KB" : (b / 1048576).toFixed(1) + " MB"); }
    function ago(s) {
        if (s < 60) return "just now";
        if (s < 3600) return Math.floor(s / 60) + " min ago";
        if (s < 86400) return Math.floor(s / 3600) + " h ago";
        var d = Math.floor(s / 86400); return d + " day" + (d === 1 ? "" : "s") + " ago";
    }
    function info(i) { return i ? bytes(i.bytes) + " · " + ago(i.secsAgo) : null; }
    function tone(d) { return d === "Easy" ? [Theme.goodFg, Theme.goodBg] : (d === "Hard" ? [Theme.badFg, Theme.badBg] : [Theme.warnFg, Theme.warnBg]); }
    function n(v) { return Theme.money(v).substring(1); }
    property string said: ""
    Timer { id: sayTimer; interval: 3500; onTriggered: root.said = "" }
    function say(t) { said = t; sayTimer.restart(); }

    component KV: RowLayout {
        property string k
        property string v
        Layout.fillWidth: true
        Text { text: k; color: Theme.inkSubtle; font.pixelSize: 13; Layout.fillWidth: true }
        Text { text: v; color: Theme.ink; font.pixelSize: 13; font.bold: true }
    }

    ScrollView {
        id: sv
        anchors.fill: parent
        contentWidth: availableWidth
        clip: true
        ColumnLayout {
            width: Math.min(sv.availableWidth - 2 * Theme.gutter, 760)
            x: Math.max(Theme.gutter, (sv.availableWidth - width) / 2)
            y: Theme.gutter
            spacing: 12
            ScreenTitle { text: "Settings" }

            Card {
                title: "Sound"
                RowLayout {
                    Layout.fillWidth: true; spacing: 10
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 0
                        Text { text: "Background music"; color: Theme.ink; font.pixelSize: 14 }
                        Text { text: Music.tracks.length === 0 ? "No tracks found." : Music.tracks.length + " tracks, shuffled"; color: Theme.inkSubtle; font.pixelSize: 12 }
                    }
                    Check { checked: Music.enabled; onToggled: Music.setEnabled(checked) }
                }
                RowLayout {
                    Layout.fillWidth: true; spacing: 8
                    Text { text: "🔈"; color: Theme.inkSubtle }
                    Slide { Layout.fillWidth: true; from: 0; to: 1; stepSize: 0.05; value: Music.volume; enabled: Music.enabled; onMoved: Music.setVolume(value); onPressedChanged: if (!pressed) Music.saveVolume() }
                    Text { text: "🔊"; color: Theme.inkSubtle }
                    Text { text: Math.round(Music.volume * 100) + "%"; color: Theme.inkMuted; font.pixelSize: 12; Layout.preferredWidth: 36 }
                }
                RowLayout {
                    visible: Music.enabled && Music.nowPlaying !== ""
                    Layout.fillWidth: true; spacing: 8
                    Text { text: "♪"; color: Theme.primary }
                    Text { text: Music.nowTitle; color: Theme.ink; font.bold: true; font.pixelSize: 13; elide: Text.ElideRight; Layout.fillWidth: true }
                    PrimaryButton { text: "Next track"; ghost: true; implicitHeight: 30; onClicked: Music.next() }
                }
                Text {
                    visible: Music.unavailable
                    text: "Audio is not available on this device right now. The game works fine without it."
                    color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true
                }
            }

            Card {
                visible: !!root.fr
                title: "Your franchise"
                KV { k: "Team"; v: root.fr ? root.fr.team : "" }
                KV { k: "Season"; v: root.fr ? root.fr.season + " · " + root.fr.phase : "" }
                KV { k: "Record"; v: root.fr ? root.fr.record : "" }
                RowLayout {
                    Layout.fillWidth: true
                    Text { text: "League seed"; color: Theme.inkSubtle; font.pixelSize: 13; Layout.fillWidth: true }
                    Text { id: seedText; text: root.fr ? (root.fr.seed === "0" ? "unknown (older save)" : root.fr.seed) : ""; color: Theme.ink; font.pixelSize: 13; font.bold: true }
                    PrimaryButton {
                        visible: !!root.fr && root.fr.seed !== "0"
                        text: "Copy"; ghost: true; implicitHeight: 26; leftPadding: 10; rightPadding: 10
                        onClicked: { seedCopy.text = root.fr.seed; seedCopy.selectAll(); seedCopy.copy(); root.say("Seed copied"); }
                    }
                    TextEdit { id: seedCopy; visible: false }
                }
                Text { text: "Same seed, same town, same choices: same league. Handy for sharing a challenge."; color: Theme.inkFaint; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider }
                Text { text: "SAVES"; color: Theme.inkSubtle; font.pixelSize: 11; font.bold: true; font.letterSpacing: 1.1 }
                KV { k: "Autosave"; v: root.info(root.vm.save) || "none yet" }
                KV { k: "Backup copy"; v: root.info(root.vm.backup) || "none yet" }
                Flow {
                    Layout.fillWidth: true; spacing: 8
                    PrimaryButton { text: "Back up now"; ghost: true; implicitHeight: 36; onClicked: { var ok = Store.game.backupSave(); root.tick++; root.say(ok ? "Backup saved" : "Could not make a backup"); } }
                    PrimaryButton { text: "Restore backup"; ghost: true; implicitHeight: 36; enabled: !!root.vm.backup; onClicked: restoreDlg.open() }
                    PrimaryButton { text: "Abandon franchise"; ghost: true; implicitHeight: 36; onClicked: abandonDlg.open() }
                }
                Text { text: "The game saves itself after every move. A backup is a second copy you can go back to."; color: Theme.inkFaint; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                Text { visible: root.said !== ""; text: root.said; color: Theme.primary; font.bold: true; font.pixelSize: 13 }
            }

            Card {
                visible: !!root.town
                title: "Difficulty"
                trailing: Pill { visible: !!root.town; text: root.town ? root.town.difficulty : ""; strong: true; fg: root.town ? root.tone(root.town.difficulty)[0] : "white"; bg: root.town ? root.tone(root.town.difficulty)[1] : "black" }
                Text {
                    text: root.town ? "Difficulty comes from your town. There is no separate setting. You picked " + root.town.name + "." : ""
                    color: Theme.inkMuted; font.pixelSize: 14; wrapMode: Text.WordWrap; Layout.fillWidth: true
                }
                KV { k: "Population"; v: root.town ? root.n(root.town.population) : "" }
                KV { k: "Stadium seats"; v: root.town ? root.n(root.town.capacity) : "" }
                KV { k: "Fan loyalty"; v: root.town ? root.town.loyalty + " / 10" : "" }
                KV { k: "Local wealth"; v: root.town ? root.town.wealth + " / 10" : "" }
                KV { k: "Local business"; v: root.town ? root.town.biz + " / 10" : "" }
                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider }
                Repeater {
                    model: [ ["Easy", "A big, rich, loyal town. Crowds and sponsors forgive mistakes."], ["Medium", "Enough money to learn on, not enough to waste."], ["Hard", "A small stadium and thin sponsors. Every payroll decision counts, and the bank is closer than you think."] ]
                    RowLayout {
                        Layout.fillWidth: true; spacing: 8
                        Text { text: modelData[0]; color: Theme.ink; font.bold: true; Layout.preferredWidth: 64; Layout.alignment: Qt.AlignTop }
                        Text { text: modelData[1]; color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    }
                }
            }

            Card {
                title: "Tutorial and help"
                RowLayout {
                    Layout.fillWidth: true
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 0
                        Text { text: "Replay the tutorial"; color: Theme.ink; font.pixelSize: 14 }
                        Text { text: "The quick tour of a game week, the roster, money and the offseason."; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    }
                    PrimaryButton { text: "Open"; ghost: true; implicitHeight: 34; onClicked: root.replayTutorial() }
                }
            }

            Card {
                title: "About"
                Text { text: "Semi-Pro Football"; color: Theme.ink; font.pixelSize: 24; font.bold: true }
                Text { text: "Version " + (root.vm.version || ""); color: Theme.inkSubtle; font.pixelSize: 12 }
                Text { text: "A small-town football management sim. You own the team, the payroll and the problems. Every game is a seeded simulation, so a franchise always plays out the same from the same choices."; color: Theme.inkMuted; font.pixelSize: 14; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider }
                Text { text: "CREDITS"; color: Theme.inkSubtle; font.pixelSize: 11; font.bold: true; font.letterSpacing: 1.1 }
                Text {
                    text: "Linux edition built with Rust, Qt 6 and QML.\nNames, jobs and size tables are data files you can edit in the assets folder."
                    color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true
                }
                Text { visible: Music.tracks.length > 0; text: "Music: " + Music.tracks.length + " tracks play in shuffled order."; color: Theme.inkMuted; font.pixelSize: 13 }
            }
            Item { Layout.preferredHeight: 8 }
        }
    }

    ConfirmDialog {
        id: restoreDlg
        title: "Restore the backup?"
        body: "Your current franchise is replaced by the copy from " + (root.vm.backup ? root.ago(root.vm.backup.secsAgo) : "") + ". Anything played since then is lost."
        action: "Restore"
        onConfirmed: { var ok = Store.game.restoreBackup(); root.tick++; root.say(ok ? "Backup restored" : "The backup could not be read"); }
    }
    ConfirmDialog {
        id: abandonDlg
        title: "Abandon this franchise?"
        body: "This deletes your current franchise. The town will be told you were a flake. A backup copy, if you made one, is kept."
        action: "Delete it"
        onConfirmed: { Store.game.abandonFranchise(); root.abandoned(); }
    }
}
