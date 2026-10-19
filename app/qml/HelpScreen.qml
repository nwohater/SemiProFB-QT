import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Every rule in one place: search, filter by category, click a topic to open it.
Item {
    id: root
    property string query: ""
    property string category: ""
    property var open: ({})
    readonly property var help: JSON.parse(Store.game.helpJson())
    readonly property string q: query.trim().toLowerCase()
    function matches(t) {
        if (category !== "" && t.category !== category) return false;
        if (q === "") return true;
        return t.title.toLowerCase().indexOf(q) >= 0 || t.body.some(function(b) { return b.toLowerCase().indexOf(q) >= 0; });
    }
    readonly property var shown: help.topics.filter(matches)
    function isOpen(t) { return q !== "" || !!open[t.title]; }
    function toggle(t) { var o = Object.assign({}, open); o[t.title] = !o[t.title]; open = o; }

    ScrollView {
        id: sv
        anchors.fill: parent
        contentWidth: availableWidth
        clip: true
        ColumnLayout {
            width: Math.min(sv.availableWidth - 2 * Theme.gutter, 860)
            x: Math.max(Theme.gutter, (sv.availableWidth - width) / 2)
            y: Theme.gutter
            spacing: 12
            ScreenTitle { text: "Help" }

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: hero.implicitHeight + 32
                radius: 16; color: Theme.primaryTint; border.color: Theme.primaryDark
                ColumnLayout {
                    id: hero
                    anchors.fill: parent; anchors.margins: 16; spacing: 6
                    Text { text: "HOW IT ALL WORKS"; color: Theme.ink; font.pixelSize: 30; font.bold: true }
                    Text { text: "Pay, hometowns, commutes, fan favorites, sponsors, the books. Open a topic, or search."; color: Theme.inkMuted; font.pixelSize: 14; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    TextField {
                        Layout.fillWidth: true; Layout.topMargin: 6
                        placeholderText: "Search help (try \"commute\")"
                        color: Theme.ink; placeholderTextColor: Theme.inkFaint
                        selectByMouse: true
                        onTextChanged: root.query = text
                        background: Rectangle { radius: 10; color: Theme.surface; border.color: parent.activeFocus ? Theme.primary : Theme.outline }
                    }
                }
            }
            Flow {
                Layout.fillWidth: true; spacing: 6
                Chip { text: "All"; selected: root.category === ""; onClicked: root.category = "" }
                Repeater { model: root.help.categories; Chip { text: modelData; selected: root.category === modelData; onClicked: root.category = modelData } }
            }
            Text { visible: root.shown.length === 0; text: "Nothing matches that. Try another word."; color: Theme.inkMuted; Layout.alignment: Qt.AlignHCenter; topPadding: 24 }

            Repeater {
                model: root.help.categories
                ColumnLayout {
                    id: catCol
                    readonly property string cat: modelData
                    readonly property var list: root.shown.filter(function(t) { return t.category === catCol.cat; })
                    visible: list.length > 0
                    Layout.fillWidth: true; spacing: 6
                    Text { text: catCol.cat.toUpperCase(); color: Theme.primary; font.pixelSize: 12; font.bold: true; font.letterSpacing: 1.1; topPadding: 8 }
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: topics.implicitHeight
                        radius: Theme.radiusCard; color: Theme.surface; border.color: Theme.divider; clip: true
                        ColumnLayout {
                            id: topics
                            width: parent.width; spacing: 0
                            Repeater {
                                model: catCol.list
                                ColumnLayout {
                                    id: topic
                                    readonly property bool expanded: root.isOpen(modelData)
                                    Layout.fillWidth: true; spacing: 0
                                    Rectangle { visible: index > 0; Layout.fillWidth: true; height: 1; color: Theme.divider }
                                    Rectangle {
                                        Layout.fillWidth: true; implicitHeight: 46
                                        color: th.hovered ? Theme.surfaceHi : "transparent"
                                        RowLayout {
                                            anchors.fill: parent; anchors.leftMargin: 14; anchors.rightMargin: 14
                                            Text { text: modelData.title; color: Theme.ink; font.pixelSize: 15; font.bold: true; Layout.fillWidth: true; elide: Text.ElideRight }
                                            Text { text: topic.expanded ? "▾" : "▸"; color: Theme.inkFaint; font.pixelSize: 14 }
                                        }
                                        HoverHandler { id: th; cursorShape: Qt.PointingHandCursor }
                                        TapHandler { onTapped: root.toggle(modelData) }
                                    }
                                    ColumnLayout {
                                        visible: topic.expanded
                                        Layout.fillWidth: true; Layout.leftMargin: 14; Layout.rightMargin: 14; Layout.bottomMargin: 14
                                        spacing: 8
                                        Repeater {
                                            model: modelData.body
                                            RowLayout {
                                                readonly property bool bullet: modelData.indexOf("- ") === 0
                                                Layout.fillWidth: true; spacing: 8
                                                Text { visible: bullet; text: "•"; color: Theme.primary; Layout.alignment: Qt.AlignTop }
                                                Text { text: bullet ? modelData.substring(2) : modelData; color: Theme.inkMuted; font.pixelSize: 14; wrapMode: Text.WordWrap; Layout.fillWidth: true; lineHeight: 1.25 }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Item { Layout.preferredHeight: 16 }
        }
    }
}
