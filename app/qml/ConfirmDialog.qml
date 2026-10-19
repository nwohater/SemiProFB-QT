import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// A themed Cancel / action confirmation.
Popup {
    id: dlg
    property string title: ""
    property string body: ""
    property string action: "OK"
    property bool danger: false
    signal confirmed()

    parent: Overlay.overlay
    anchors.centerIn: parent
    modal: true
    focus: true
    width: Math.min(parent ? parent.width - 32 : 420, 420)
    padding: 18
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    background: Rectangle { color: Theme.surface; radius: Theme.radiusCard + 2; border.color: Theme.outline }
    Overlay.modal: Rectangle { color: "#aa0b0b12" }

    contentItem: ColumnLayout {
        spacing: 12
        Text { text: dlg.title; color: Theme.ink; font.pixelSize: 17; font.bold: true; wrapMode: Text.WordWrap; Layout.fillWidth: true }
        Text { text: dlg.body; color: Theme.inkMuted; font.pixelSize: 14; wrapMode: Text.WordWrap; Layout.fillWidth: true }
        RowLayout {
            Layout.alignment: Qt.AlignRight; spacing: 8
            PrimaryButton { text: "Cancel"; ghost: true; implicitHeight: 38; onClicked: dlg.close() }
            PrimaryButton { text: dlg.action; implicitHeight: 38; onClicked: { dlg.close(); dlg.confirmed(); } }
        }
    }
}
