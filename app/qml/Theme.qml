pragma Singleton
import QtQuick

// Dark theme (Tokyo-night inspired) with the structure of the Flutter design tokens: surfaces, ink, one primary,
// and Good / Okay / Risky status colors that always travel with a word.
QtObject {
    // surfaces
    readonly property color bg: "#16161e"
    readonly property color surface: "#1f2335"
    readonly property color surfaceAlt: "#24283b"
    readonly property color surfaceHi: "#2a2f45"
    readonly property color divider: "#2f3549"
    readonly property color outline: "#3b4261"
    // ink
    readonly property color ink: "#c0caf5"
    readonly property color inkMuted: "#a9b1d6"
    readonly property color inkSubtle: "#7d86a8"
    readonly property color inkFaint: "#565f89"
    // brand
    readonly property color primary: "#7aa2f7"
    readonly property color primaryDark: "#3d59a1"
    readonly property color primaryTint: "#283457"
    readonly property color gold: "#e0af68"
    // status
    readonly property color goodFg: "#9ece6a"
    readonly property color goodBg: "#233324"
    readonly property color warnFg: "#e0af68"
    readonly property color warnBg: "#3a3020"
    readonly property color badFg: "#f7768e"
    readonly property color badBg: "#3b2230"
    // shape
    readonly property int radiusCard: 12
    readonly property int radiusButton: 10
    readonly property int radiusChip: 8
    readonly property int radiusPill: 999
    readonly property int gutter: 16
    // type
    readonly property var displayFamilies: ["Barlow Condensed", "Oswald", "Bebas Neue", "Arial Narrow", "sans-serif"]
    readonly property var bodyFamilies: ["IBM Plex Sans", "Inter", "Noto Sans", "sans-serif"]

    function money(v) {
        var n = Math.round(Math.abs(v));
        var s = String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
        return (v < 0 ? "-$" : "$") + s;
    }
    function fitColor(fit) { return fit === "Good" ? goodFg : (fit === "Risky" ? badFg : warnFg); }
    function fitBg(fit) { return fit === "Good" ? goodBg : (fit === "Risky" ? badBg : warnBg); }
    // Same bands the Help text promises: green 65 and up, amber 45 to 64, red below 45.
    function ovrColor(o) { return o >= 65 ? goodFg : (o >= 45 ? warnFg : badFg); }
}
