import QtQuick

// Good / Okay / Risky, always with the word. `note` replaces the word with a short part text ("QB arm 71").
Rectangle {
    id: chip
    property string fit: "Okay"
    property string note: ""
    color: Theme.fitBg(fit)
    radius: Theme.radiusChip - 2
    implicitHeight: 24
    implicitWidth: label.implicitWidth + 16
    Text {
        id: label
        anchors.centerIn: parent
        text: (chip.fit === "Good" ? "✓ " : (chip.fit === "Risky" ? "⚠ " : "– ")) + (chip.note !== "" ? chip.note : chip.fit)
        color: Theme.fitColor(chip.fit)
        font.pixelSize: 12; font.bold: true
    }
}
