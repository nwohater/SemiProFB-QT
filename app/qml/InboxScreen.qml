import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    property string feed: "all"
    property int openId: -1
    function toggle(m) { if (openId === m.id) { openId = -1; return; } openId = m.id; if (m.unread) Store.game.markItemRead(m.id); }
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.inboxJson()); }
    readonly property var items: (vm.items || []).filter(function(i) { return feed === "all" || (feed === "unread" ? (i.unread || i.id === openId) : i.feed === feed); })

    Component.onCompleted: { var a = Qt.application.arguments.indexOf("--dev-mail"); if (a >= 0 && items.length) toggle(items[parseInt(Qt.application.arguments[a + 1])]); }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.gutter
        spacing: 10
        RowLayout {
            Layout.fillWidth: true
            ScreenTitle { text: "Inbox"; Layout.fillWidth: true }
            PrimaryButton { text: "Mark all read"; ghost: true; implicitHeight: 34; onClicked: Store.game.markInboxRead() }
        }
        RowLayout {
            spacing: 6
            Chip { text: "All"; selected: root.feed === "all"; onClicked: root.feed = "all" }
            Chip { text: "Unread" + (root.vm.unreadTotal ? " (" + root.vm.unreadTotal + ")" : ""); selected: root.feed === "unread"; onClicked: root.feed = "unread" }
            Chip { text: "Money" + (root.vm.unreadMoney ? " (" + root.vm.unreadMoney + ")" : ""); selected: root.feed === "money"; onClicked: root.feed = "money" }
            Chip { text: "Travel" + (root.vm.unreadTravel ? " (" + root.vm.unreadTravel + ")" : ""); selected: root.feed === "travel"; onClicked: root.feed = "travel" }
        }
        Text { visible: root.items.length === 0; text: root.feed === "unread" ? "Nothing unread." : "Nothing here yet."; color: Theme.inkSubtle; font.pixelSize: 14; topPadding: 12 }
        // A mail list: click a subject and the message opens below it.
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 0
            model: root.items
            ScrollBar.vertical: ScrollBar {}
            delegate: Rectangle {
                id: mail
                readonly property bool expanded: root.openId === modelData.id
                width: ListView.view.width - 8
                implicitHeight: col.implicitHeight
                height: implicitHeight
                color: expanded ? Theme.surface : (rowHover.hovered ? Theme.surfaceAlt : "transparent")
                border.color: expanded ? Theme.outline : "transparent"
                radius: expanded ? Theme.radiusChip : 0
                ColumnLayout {
                    id: col
                    width: parent.width
                    spacing: 0
                    // header row
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.margins: 10
                        Layout.topMargin: 9; Layout.bottomMargin: 9
                        spacing: 10
                        Rectangle { width: 8; height: 8; radius: 4; color: modelData.unread ? Theme.primary : "transparent"; Layout.alignment: Qt.AlignVCenter }
                        Text {
                            text: modelData.from ? modelData.from : (modelData.feed === "money" ? "Club office" : "Road crew")
                            color: modelData.unread ? Theme.ink : Theme.inkSubtle; font.pixelSize: 13; font.bold: modelData.unread
                            Layout.preferredWidth: 150; elide: Text.ElideRight
                        }
                        Text {
                            text: modelData.title
                            color: modelData.unread ? Theme.ink : Theme.inkMuted; font.pixelSize: 14; font.bold: modelData.unread
                            Layout.fillWidth: true; elide: Text.ElideRight
                        }
                        Text { visible: !mail.expanded && root.width > 700; text: modelData.text.replace(/\s+/g, " "); color: Theme.inkFaint; font.pixelSize: 12; Layout.preferredWidth: Math.min(360, root.width * 0.3); elide: Text.ElideRight }
                        Text { text: "S" + modelData.season + " · Wk " + modelData.week; color: Theme.inkFaint; font.pixelSize: 11 }
                        TapHandler { onTapped: root.toggle(modelData) }
                        HoverHandler { id: rowHover; cursorShape: Qt.PointingHandCursor }
                    }
                    // the message
                    ColumnLayout {
                        visible: mail.expanded
                        Layout.fillWidth: true
                        Layout.leftMargin: 28; Layout.rightMargin: 14; Layout.bottomMargin: 14
                        spacing: 8
                        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider }
                        Text { visible: !!modelData.from; text: "From " + modelData.from; color: Theme.inkSubtle; font.pixelSize: 12 }
                        Text { text: modelData.text; color: Theme.inkMuted; font.pixelSize: 14; wrapMode: Text.WordWrap; Layout.fillWidth: true; lineHeight: 1.2; font.italic: !!modelData.from }
                        // Plea for help from a struggling club
                    Text {
                        visible: modelData.pleaState === "Helped" && modelData.pleaAsk > 0
                        text: "You sent the " + modelData.pleaTeam + " help."
                        color: Theme.primary; font.pixelSize: 13; font.bold: true
                    }
                    Text {
                        visible: modelData.pleaState === "Declined"
                        text: "You passed on the " + modelData.pleaTeam + "."
                        color: Theme.inkSubtle; font.pixelSize: 13
                    }
                    Flow {
                        visible: modelData.pleaOpen
                        Layout.fillWidth: true
                        spacing: 8
                        readonly property int half: Math.round(modelData.pleaAsk / 2 / 500) * 500
                        PrimaryButton {
                            text: "Give " + Theme.money(modelData.pleaAsk)
                            enabled: root.vm.cash >= modelData.pleaAsk
                            implicitHeight: 38
                            onClicked: Store.game.helpClub(modelData.id, modelData.pleaAsk)
                        }
                        PrimaryButton {
                            visible: parent.half >= 500
                            text: "Give " + Theme.money(parent.half)
                            ghost: true
                            enabled: root.vm.cash >= parent.half
                            implicitHeight: 38
                            onClicked: Store.game.helpClub(modelData.id, parent.half)
                        }
                        PrimaryButton { text: "Not this time"; ghost: true; implicitHeight: 38; onClicked: Store.game.declinePlea(modelData.id) }
                    }

                    // Off-field trouble
                    Text {
                        visible: modelData.conductState === "Paid"
                        text: "You paid " + Theme.money(modelData.conductCost) + ". It went away."
                        color: Theme.primary; font.pixelSize: 13; font.bold: true
                    }
                    Text {
                        visible: modelData.isConduct && modelData.conductState === "Sits"
                        text: "You let him face it. He sits " + modelData.conductGames + " game" + (modelData.conductGames === 1 ? "" : "s") + " and the town talks."
                        color: Theme.badFg; font.pixelSize: 13; font.bold: true
                    }
                    Flow {
                        visible: modelData.conductOpen
                        Layout.fillWidth: true
                        spacing: 8
                        PrimaryButton {
                            text: "Pay " + Theme.money(modelData.conductCost)
                            enabled: root.vm.cash >= modelData.conductCost
                            implicitHeight: 38
                            onClicked: Store.game.payConduct(modelData.id)
                        }
                        PrimaryButton {
                            text: "Let him sit " + modelData.conductGames + " game" + (modelData.conductGames === 1 ? "" : "s")
                            ghost: true
                            implicitHeight: 38
                            onClicked: Store.game.letHimSit(modelData.id)
                        }
                    }
                    Text {
                        visible: modelData.conductOpen
                        text: "Paying keeps it quiet. If he sits, fans hear about it and interest drops. Do nothing and he sits when the next game is played."
                        color: Theme.inkFaint; font.pixelSize: 11; wrapMode: Text.WordWrap; Layout.fillWidth: true
                    }
                    PrimaryButton {
                        readonly property var tabFor: ({ "Sponsor": "Sponsors", "Bank": "Loan", "Council": "Council", "Promo": "Tickets" })
                        visible: tabFor[modelData.kind] !== undefined
                        text: modelData.kind === "Sponsor" ? "See sponsor offers" : (modelData.kind === "Bank" ? "Open loan" : (modelData.kind === "Council" ? "Open council" : "Open tickets and promos"))
                        implicitHeight: 32
                        onClicked: Store.openFinanceTab(tabFor[modelData.kind])
                    }
                    PrimaryButton {
                        visible: modelData.playerId !== null && modelData.playerId !== undefined
                        text: "View player"; ghost: true; implicitHeight: 32
                        onClicked: Store.openPlayer(modelData.playerId)
                    }
                    }
                    Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider; visible: !mail.expanded }
                }
            }
        }
    }
}
