import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// A sortable stat table: player column, then one column per stat. Tap a header to sort, tap a row to open the player.
// `table` is the view-model from stats.rs: { cols: [{header, width}], rows: [{id, name, team, pos, mine, cells: [{t, v}]}], sortCol }.
Item {
    id: root
    property var table: ({ cols: [], rows: [], sortCol: 0 })
    property string empty: "No numbers yet."
    property int pageSize: 25
    property bool showTeam: true
    property int sortIdx: -1
    property bool desc: true
    property int shown: pageSize
    readonly property int nameW: showTeam ? 190 : 150
    readonly property int rowH: 36
    readonly property var cols: table.cols || []
    readonly property int sortCol: sortIdx >= 0 ? Math.min(sortIdx, cols.length - 1) : (table.sortCol || 0)
    readonly property var sorted: {
        var rows = (table.rows || []).slice();
        var s = sortCol;
        rows.sort(function(a, b) { var d = a.cells[s].v - b.cells[s].v; return desc ? -d : d; });
        return rows;
    }
    readonly property int statsW: { var w = 0; for (var i = 0; i < cols.length; i++) w += cols[i].width; return w; }
    onTableChanged: { sortIdx = -1; desc = true; shown = pageSize; }
    implicitHeight: (sorted.length === 0 ? 40 : rowH * (Math.min(shown, sorted.length) + 1) + (sorted.length > shown ? 44 : 0))

    Text { visible: root.sorted.length === 0; text: root.empty; color: Theme.inkSubtle; font.pixelSize: 13; anchors.left: parent.left; anchors.top: parent.top; anchors.topMargin: 8 }

    Flickable {
        id: flick
        visible: root.sorted.length > 0
        anchors.fill: parent
        anchors.bottomMargin: root.sorted.length > root.shown ? 44 : 0
        contentWidth: Math.max(width, root.nameW + root.statsW)
        contentHeight: col.implicitHeight
        interactive: contentWidth > width
        flickableDirection: Flickable.HorizontalFlick
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        ColumnLayout {
            id: col
            width: flick.contentWidth   // (parent is the content item, which has no contentWidth: the rows had zero width and nothing to tap)
            spacing: 0
            // header
            Rectangle {
                Layout.fillWidth: true; Layout.preferredHeight: root.rowH
                color: Theme.surfaceHi
                RowLayout {
                    anchors.fill: parent; spacing: 0
                    Item {
                        Layout.preferredWidth: root.nameW; Layout.fillHeight: true
                        Text { text: "Player"; color: Theme.inkSubtle; font.pixelSize: 12; font.bold: true; x: 10; anchors.verticalCenter: parent.verticalCenter }
                    }
                    Repeater {
                        model: root.cols
                        Rectangle {
                            Layout.preferredWidth: modelData.width; Layout.fillHeight: true
                            color: "transparent"
                            Text {
                                anchors.centerIn: parent
                                text: modelData.header + (root.sortCol === index ? (root.desc ? " ▾" : " ▴") : "")
                                color: root.sortCol === index ? Theme.primary : Theme.inkSubtle
                                font.pixelSize: 12; font.bold: true
                            }
                            TapHandler { onTapped: { if (root.sortCol === index) root.desc = !root.desc; else { root.sortIdx = index; root.desc = true; } } }
                            HoverHandler { cursorShape: Qt.PointingHandCursor }
                        }
                    }
                }
            }
            Repeater {
                model: root.sorted.slice(0, root.shown)
                Rectangle {
                    Layout.fillWidth: true; Layout.preferredHeight: root.rowH
                    color: rh.hovered ? Theme.surfaceHi : (index % 2 === 0 ? "transparent" : Qt.alpha(Theme.surfaceAlt, 0.5))
                    RowLayout {
                        anchors.fill: parent; spacing: 0
                        Item {
                            Layout.preferredWidth: root.nameW; Layout.fillHeight: true
                            Text {
                                x: 10; width: parent.width - 10 - (root.showTeam ? 62 : 34); anchors.verticalCenter: parent.verticalCenter
                                text: modelData.name; color: modelData.mine ? Theme.primary : Theme.ink; font.bold: modelData.mine; font.pixelSize: 13; elide: Text.ElideRight
                            }
                            Text { visible: root.showTeam; x: parent.width - 62; anchors.verticalCenter: parent.verticalCenter; text: modelData.team; color: Theme.inkFaint; font.pixelSize: 11 }
                            Text { x: parent.width - 32; anchors.verticalCenter: parent.verticalCenter; text: modelData.pos; color: Theme.inkSubtle; font.pixelSize: 11 }
                        }
                        Repeater {
                            model: modelData.cells
                            Text {
                                Layout.preferredWidth: root.cols[index] ? root.cols[index].width : 54
                                horizontalAlignment: Text.AlignHCenter
                                text: modelData.t
                                color: root.sortCol === index ? Theme.ink : Theme.inkMuted
                                font.pixelSize: 13; font.bold: root.sortCol === index
                            }
                        }
                    }
                    MouseArea {
                        id: rh
                        anchors.fill: parent; hoverEnabled: true
                        property bool hovered: containsMouse
                        cursorShape: modelData.team === "Ret" ? Qt.ArrowCursor : Qt.PointingHandCursor
                        onClicked: { if (modelData.team !== "Ret") Store.openPlayer(modelData.id) }
                    }
                }
            }
        }
    }
    PrimaryButton {
        visible: root.sorted.length > root.shown
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        text: "Show more (" + (root.sorted.length - root.shown) + ")"
        ghost: true; implicitHeight: 34
        onClicked: root.shown += root.pageSize
    }
}
