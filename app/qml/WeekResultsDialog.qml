import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// After advancing: every game of the week, yours first, plus any day-job news.
Popup {
    id: dlg
    property var res: ({ games: [], workNews: [] })
    function show(r) { res = r; open(); }

    modal: true
    focus: true
    anchors.centerIn: parent
    width: Math.min(parent ? parent.width - 32 : 520, 560)
    height: Math.min(parent ? parent.height - 48 : 600, 640)
    padding: 0
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    background: Rectangle { color: Theme.surface; radius: Theme.radiusCard + 2; border.color: Theme.outline; border.width: 1 }
    Overlay.modal: Rectangle { color: "#aa0b0b12" }

    contentItem: ColumnLayout {
        spacing: 0
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 56
            color: "transparent"
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16; anchors.rightMargin: 16
                ScreenTitle { text: dlg.res.label || "Results"; font.pixelSize: 24; Layout.fillWidth: true }
                PrimaryButton { text: "Continue"; implicitHeight: 36; onClicked: dlg.close() }
            }
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.divider }
        }
        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true
            ColumnLayout {
                width: parent.width
                spacing: 8
                Item { Layout.preferredHeight: 4 }
                Repeater {
                    model: (dlg.res.games || []).slice().sort(function(a, b) { return (b.mine ? 1 : 0) - (a.mine ? 1 : 0); })
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.leftMargin: 12; Layout.rightMargin: 12
                        implicitHeight: 58
                        radius: Theme.radiusChip + 2
                        color: modelData.mine ? Theme.primaryTint : Theme.surfaceAlt
                        border.color: modelData.mine ? Theme.primary : "transparent"
                        border.width: 1
                        TapHandler { enabled: modelData.box !== null; onTapped: Store.openBox(modelData.box) }
                        HoverHandler { cursorShape: modelData.box !== null ? Qt.PointingHandCursor : Qt.ArrowCursor }
                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 8
                            Helmet { slug: modelData.away.helmet; size: 34 }
                            Text { text: modelData.away.name; color: modelData.as > modelData.hs ? Theme.ink : Theme.inkSubtle; font.bold: modelData.as > modelData.hs; font.pixelSize: 13; elide: Text.ElideRight; Layout.fillWidth: true }
                            Text { text: modelData.as; color: modelData.as > modelData.hs ? Theme.ink : Theme.inkSubtle; font.pixelSize: 20; font.bold: true; Layout.preferredWidth: 34; horizontalAlignment: Text.AlignRight }
                            Text { text: "@"; color: Theme.inkFaint; font.pixelSize: 12 }
                            Text { text: modelData.hs; color: modelData.hs > modelData.as ? Theme.ink : Theme.inkSubtle; font.pixelSize: 20; font.bold: true; Layout.preferredWidth: 34 }
                            Text { text: modelData.home.name + (modelData.ot ? "  (OT)" : ""); color: modelData.hs > modelData.as ? Theme.ink : Theme.inkSubtle; font.bold: modelData.hs > modelData.as; font.pixelSize: 13; elide: Text.ElideRight; Layout.fillWidth: true }
                            Helmet { slug: modelData.home.helmet; size: 34 }
                        }
                    }
                }
                Card {
                    visible: (dlg.res.workNews || []).length > 0
                    Layout.leftMargin: 12; Layout.rightMargin: 12
                    title: "Day jobs"
                    Repeater { model: dlg.res.workNews || []; Text { text: modelData; color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true } }
                }
                Item { Layout.preferredHeight: 8 }
            }
        }
    }
}
