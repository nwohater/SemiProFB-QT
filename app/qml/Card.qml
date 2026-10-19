import QtQuick
import QtQuick.Layouts

// A titled card: small heading, optional trailing item, content below. Dividers instead of shadows.
Rectangle {
    id: card
    property string title: ""
    property alias trailing: trailingSlot.data
    default property alias content: body.data
    property int pad: 14
    property color fill: Theme.surface

    color: fill
    radius: Theme.radiusCard
    border.color: Theme.divider
    border.width: 1
    implicitHeight: col.implicitHeight + pad * 2
    Layout.fillWidth: true

    ColumnLayout {
        id: col
        anchors.fill: parent
        anchors.margins: card.pad
        spacing: 10
        RowLayout {
            visible: card.title !== ""
            Layout.fillWidth: true
            Text {
                text: card.title
                color: Theme.inkSubtle
                font.pixelSize: 12
                font.bold: true
                font.capitalization: Font.AllUppercase
                font.letterSpacing: 0.8
                Layout.fillWidth: true
                elide: Text.ElideRight
            }
            Item { id: trailingSlot; implicitWidth: childrenRect.width; implicitHeight: childrenRect.height }
        }
        ColumnLayout { id: body; Layout.fillWidth: true; spacing: 8 }
    }
}
