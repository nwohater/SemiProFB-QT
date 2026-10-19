import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Offseason flow: progression report, contracts, free agents, the draft, then the next season.
Item {
    id: root
    property string tab: "Progression"
    property bool gainers: true
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.offseasonJson()); }
    readonly property var budget: vm.budget || ({})
    readonly property bool draftPending: !!vm.draftPending
    readonly property var changes: {
        var c = (vm.changes || []).slice();
        c.sort(function(a, b) { return gainers ? b.delta - a.delta : a.delta - b.delta; });
        return c;
    }

    function startSeason() {
        var r = JSON.parse(Store.game.finishOffseason());
        notes = [];
        for (var i = 0; i < (r.replaced || []).length; i++) notes.push(r.replaced[i][0] + " went out of business. " + r.replaced[i][1] + " joins the league.");
        if ((r.autoSigned || []).length > 0) notes.push("Your roster could not field a team, so " + r.autoSigned.length + " walk-on(s) were signed off the diner counter.");
        if (notes.length > 0) { seasonDlg.season = r.season; seasonDlg.open(); }
    }
    property var notes: []
    Component.onCompleted: { var i = Qt.application.arguments.indexOf("--dev-otab"); if (i >= 0) tab = Qt.application.arguments[i + 1]; }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.gutter
        anchors.bottomMargin: 0
        spacing: 10
        ScreenTitle { text: "Offseason " + (root.vm.season || "") }

        // live money snapshot
        Card {
            fill: Theme.primaryTint
            GridLayout {
                Layout.fillWidth: true
                columns: root.width >= 720 ? 5 : 2
                columnSpacing: 10; rowSpacing: 8
                StatTile { label: "Cash on hand"; value: root.budget.cashText || ""; accent: root.budget.cash < 0 ? Theme.badFg : Theme.goodFg }
                StatTile { label: "Salary / game"; value: root.budget.payrollText || ""; accent: Theme.warnFg }
                StatTile { label: "Signed"; value: (root.budget.signed || 0) + " · " + (root.budget.active || 0) + "/46" }
                StatTile { visible: !!root.budget.known; label: "Net / game"; value: (root.budget.projectionPositive ? "+" : "−") + (root.budget.netText || ""); accent: root.budget.projectionPositive ? Theme.goodFg : Theme.badFg }
                StatTile { visible: !!root.budget.known; label: "Season projection"; value: (root.budget.projectionPositive ? "+" : "−") + (root.budget.projection || ""); accent: root.budget.projectionPositive ? Theme.goodFg : Theme.badFg }
            }
            Text {
                text: (root.vm.waiting || []).length === 0 ? "Nobody waiting on a new deal." : (root.vm.waiting.length + " unsigned (not counted). Revenue and costs use last season's per-game averages.")
                color: (root.vm.waiting || []).length === 0 ? Theme.inkSubtle : Theme.warnFg; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true
            }
        }

        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater { model: ["Progression", "Contracts", "Free agents", "Draft"]; Chip { text: modelData + (modelData === "Contracts" && (root.vm.expiring || 0) > 0 ? " (" + root.vm.expiring + ")" : ""); selected: root.tab === modelData; onClicked: root.tab = modelData } }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: ["Progression", "Contracts", "Free agents", "Draft"].indexOf(root.tab)

            // ---- progression
            ScrollView {
                id: progSv
                contentWidth: availableWidth
                clip: true
                ColumnLayout {
                    width: progSv.availableWidth - 8
                    spacing: 10
                    Card {
                        visible: (root.vm.retired || []).length > 0
                        title: "Retired"
                        Text { text: (root.vm.retired || []).join(", "); color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    }
                    Card {
                        visible: (root.vm.fanNews || []).length > 0
                        title: "Fan favorites"
                        Repeater { model: root.vm.fanNews || []; Text { text: "★  " + modelData; color: Theme.gold; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true } }
                    }
                    RowLayout {
                        spacing: 6
                        Chip { text: "Biggest gainers"; selected: root.gainers; onClicked: root.gainers = true }
                        Chip { text: "Biggest decliners"; selected: !root.gainers; onClicked: root.gainers = false }
                    }
                    Repeater {
                        model: root.changes
                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: pcol.implicitHeight + 20
                            radius: Theme.radiusChip + 2
                            color: hv.hovered ? Theme.surfaceAlt : Theme.surface
                            border.color: Theme.divider
                            HoverHandler { id: hv; cursorShape: Qt.PointingHandCursor }
                            TapHandler { onTapped: Store.openPlayer(modelData.id) }
                            ColumnLayout {
                                id: pcol
                                anchors.fill: parent; anchors.margins: 10
                                spacing: 6
                                RowLayout {
                                    spacing: 10
                                    Rectangle { width: 38; height: 38; radius: 9; color: Theme.surfaceAlt; Text { anchors.centerIn: parent; text: modelData.ovr; color: Theme.ovrColor(modelData.ovr); font.bold: true; font.pixelSize: 16 } }
                                    ColumnLayout {
                                        Layout.fillWidth: true; spacing: 0
                                        Text { text: modelData.name; color: Theme.ink; font.pixelSize: 14; font.bold: true }
                                        Text { text: modelData.pos + " · " + modelData.age + " · OVR " + modelData.before + " → " + modelData.ovr; color: Theme.inkSubtle; font.pixelSize: 12 }
                                    }
                                    Text { text: modelData.delta > 0 ? "▲ " + modelData.delta : (modelData.delta < 0 ? "▼ " + (-modelData.delta) : "= 0"); color: modelData.delta > 0 ? Theme.goodFg : (modelData.delta < 0 ? Theme.badFg : Theme.inkSubtle); font.bold: true; font.pixelSize: 15 }
                                }
                                Flow {
                                    Layout.fillWidth: true; spacing: 10
                                    Repeater { model: modelData.attrs; Text { text: modelData.code + " " + modelData.from + "→" + modelData.to; font.pixelSize: 12; font.bold: true; color: modelData.to > modelData.from ? Theme.goodFg : (modelData.to < modelData.from ? Theme.badFg : Theme.inkSubtle) } }
                                }
                            }
                        }
                    }
                    Text { visible: (root.vm.changes || []).length === 0; text: "No changes recorded."; color: Theme.inkSubtle }
                    Item { Layout.preferredHeight: 6 }
                }
            }

            // ---- contracts
            ScrollView {
                id: conSv
                contentWidth: availableWidth
                clip: true
                ColumnLayout {
                    width: conSv.availableWidth - 8
                    spacing: 8
                    Text { text: "Re-sign them or they walk when the season starts. Asks reflect their new ratings."; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    Text { visible: (root.vm.waiting || []).length === 0; text: "Nobody is waiting on a new deal."; color: Theme.inkMuted }
                    Repeater {
                        model: root.vm.waiting || []
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 62; radius: Theme.radiusChip + 2; color: hv2.hovered ? Theme.surfaceAlt : Theme.surface; border.color: Theme.divider
                            HoverHandler { id: hv2; cursorShape: Qt.PointingHandCursor }
                            TapHandler { onTapped: Store.openPlayer(modelData.id) }
                            RowLayout {
                                anchors.fill: parent; anchors.margins: 10; spacing: 10
                                Rectangle { width: 40; height: 40; radius: 10; color: Theme.surfaceAlt; Text { anchors.centerIn: parent; text: modelData.ovr; color: Theme.ovrColor(modelData.ovr); font.bold: true; font.pixelSize: 17 } }
                                ColumnLayout {
                                    Layout.fillWidth: true; spacing: 0
                                    Text { text: modelData.name; color: Theme.ink; font.bold: true; font.pixelSize: 14; elide: Text.ElideRight; Layout.fillWidth: true }
                                    Text { text: modelData.pos + " · " + modelData.age + " · asks about " + modelData.askText + "/game on 3 years" + (modelData.wantsToPlay ? " · wants to play" : ""); color: Theme.inkSubtle; font.pixelSize: 12; elide: Text.ElideRight; Layout.fillWidth: true }
                                }
                                PrimaryButton { text: "Re-sign"; implicitHeight: 32; font.pixelSize: 13; onClicked: Store.openNegotiation(modelData.id) }
                            }
                        }
                    }
                }
            }

            FreeAgentsScreen { showTitle: false }
            DraftPanel {}
        }

        // bottom action
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 62
            color: "transparent"
            PrimaryButton {
                anchors.centerIn: parent
                width: Math.min(parent.width, 420)
                implicitHeight: 46
                text: root.draftPending ? "On to the draft" : "Start next season"
                onClicked: {
                    if (root.draftPending) { if ((root.vm.expiring || 0) > 0) toDraftDlg.open(); else root.tab = "Draft"; }
                    else if ((root.vm.expiring || 0) > 0) walkDlg.open();
                    else root.startSeason();
                }
            }
        }
    }

    Dialog {
        id: toDraftDlg
        parent: Overlay.overlay; anchors.centerIn: parent; modal: true
        title: "Move on to the draft?"
        standardButtons: Dialog.Cancel | Dialog.Ok
        onAccepted: root.tab = "Draft"
        contentItem: Text { text: (root.vm.expiring || 0) + " expiring player(s) have not been re-signed. They leave when the season starts. You can come back to Contracts until then."; color: Theme.inkMuted; wrapMode: Text.WordWrap; width: 320 }
        background: Rectangle { color: Theme.surface; radius: Theme.radiusCard; border.color: Theme.outline }
        header: Text { text: toDraftDlg.title; color: Theme.ink; font.bold: true; font.pixelSize: 16; padding: 16 }
    }
    Dialog {
        id: walkDlg
        parent: Overlay.overlay; anchors.centerIn: parent; modal: true
        title: "Let them walk?"
        standardButtons: Dialog.Cancel | Dialog.Ok
        onAccepted: root.startSeason()
        contentItem: Text { text: (root.vm.expiring || 0) + " expiring player(s) have not been re-signed. They will leave."; color: Theme.inkMuted; wrapMode: Text.WordWrap; width: 320 }
        background: Rectangle { color: Theme.surface; radius: Theme.radiusCard; border.color: Theme.outline }
        header: Text { text: walkDlg.title; color: Theme.ink; font.bold: true; font.pixelSize: 16; padding: 16 }
    }
    Dialog {
        id: seasonDlg
        property int season: 0
        parent: Overlay.overlay; anchors.centerIn: parent; modal: true
        title: "Season " + season + " begins"
        standardButtons: Dialog.Ok
        contentItem: Text { text: root.notes.join("\n\n"); color: Theme.inkMuted; wrapMode: Text.WordWrap; width: 340 }
        background: Rectangle { color: Theme.surface; radius: Theme.radiusCard; border.color: Theme.outline }
        header: Text { text: seasonDlg.title; color: Theme.ink; font.bold: true; font.pixelSize: 16; padding: 16 }
    }
}
