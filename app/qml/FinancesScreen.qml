import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    property string tab: "Overview"
    property bool allPay: false
    readonly property var tabs: ["Overview", "Tickets", "Sponsors", "Facilities", "Payroll", "Loan", "Council", "Sheet"]
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.financesJson()); }
    readonly property var tk: vm.tickets || ({})
    readonly property var sp: vm.sponsors || ({})
    readonly property var fc: vm.facilities || ({})
    readonly property var pr: vm.payroll || ({})
    readonly property var ln: vm.loanInfo || ({})
    readonly property var co: vm.council || ({})
    function tone(t) { return t === "good" ? [Theme.goodFg, Theme.goodBg] : (t === "bad" ? [Theme.badFg, Theme.badBg] : [Theme.warnFg, Theme.warnBg]); }
    Component.onCompleted: { var i = Qt.application.arguments.indexOf("--dev-ftab"); if (i >= 0) tab = Qt.application.arguments[i + 1]; }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.gutter
        anchors.bottomMargin: 0
        spacing: 10
        ScreenTitle { text: "Finances" }
        Card {
            fill: Theme.primaryTint
            Flow {
                Layout.fillWidth: true
                spacing: 10
                Text { text: root.vm.cashText || ""; color: root.vm.cash < 0 ? Theme.badFg : Theme.ink; font.pixelSize: 30; font.bold: true }
                Pill { text: "Season " + (root.vm.seasonNet >= 0 ? "+" : "−") + (root.vm.seasonNetText || ""); strong: true; fg: root.vm.seasonNet >= 0 ? Theme.goodFg : Theme.badFg; bg: root.vm.seasonNet >= 0 ? Theme.goodBg : Theme.badBg }
                Pill { text: root.vm.status || ""; strong: true; fg: root.tone(root.vm.statusTone)[0]; bg: root.tone(root.vm.statusTone)[1] }
                Pill { visible: root.vm.loan > 0; text: "Loan " + (root.vm.loanText || "") }
            }
            Flow {
                Layout.fillWidth: true
                spacing: 6
                Repeater { model: root.tabs; Chip { text: modelData; selected: root.tab === modelData; onClicked: root.tab = modelData } }
            }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: root.tabs.indexOf(root.tab)

            // ---------------- overview
            ScrollView {
                id: ovSv; contentWidth: availableWidth; clip: true
                ColumnLayout {
                    width: ovSv.availableWidth - 8; spacing: 10
                    RowLayout {
                        Layout.fillWidth: true; spacing: 10
                        StatTile { label: "Last game"; value: root.vm.lastGame ? (root.vm.lastGameNet >= 0 ? "+" : "") + root.vm.lastGame : "–" }
                        StatTile { label: "Avg crowd"; value: root.vm.avgCrowd ? String(root.vm.avgCrowd) : "–" }
                        StatTile { label: "Fan interest"; value: (root.vm.fanInterest || 0) + "/100" }
                    }
                    Card { visible: !root.vm.hasGames; Text { text: "Play a game and the books will fill in."; color: Theme.inkMuted } }
                    GridLayout {
                        visible: !!root.vm.hasGames
                        Layout.fillWidth: true
                        columns: root.width >= 800 ? 2 : 1; columnSpacing: 10; rowSpacing: 10
                        Card { title: "Revenue, season to date"; Layout.alignment: Qt.AlignTop; Layout.preferredWidth: 1
                            trailing: Text { text: root.vm.revenueTotal || ""; color: Theme.goodFg; font.bold: true }
                            BarList { Layout.fillWidth: true; items: root.vm.revenue || [] } }
                        Card { title: "Expenses, season to date"; Layout.alignment: Qt.AlignTop; Layout.preferredWidth: 1
                            trailing: Text { text: root.vm.expensesTotal || ""; color: Theme.badFg; font.bold: true }
                            BarList { Layout.fillWidth: true; items: root.vm.expenses || []; barColor: "#b5503a" } }
                    }
                }
            }

            // ---------------- tickets
            ScrollView {
                id: tkSv; contentWidth: availableWidth; clip: true
                ColumnLayout {
                    width: tkSv.availableWidth - 8; spacing: 10
                    Card {
                        title: "Ticket price"
                        RowLayout {
                            Layout.fillWidth: true
                            PrimaryButton { text: "−"; ghost: true; implicitHeight: 38; implicitWidth: 44; leftPadding: 4; rightPadding: 4; enabled: root.tk.price > 1; onClicked: Store.game.setTicketPrice(Math.round((root.tk.price - 0.25) * 100)) }
                            Text { text: "$" + (root.tk.priceText || "") + " per seat"; color: Theme.ink; font.pixelSize: 24; font.bold: true; horizontalAlignment: Text.AlignHCenter; Layout.fillWidth: true }
                            PrimaryButton { text: "+"; ghost: true; implicitHeight: 38; implicitWidth: 44; leftPadding: 4; rightPadding: 4; enabled: root.tk.price < 25; onClicked: Store.game.setTicketPrice(Math.round((root.tk.price + 0.25) * 100)) }
                        }
                        Slide { Layout.fillWidth: true; from: 1; to: 25; stepSize: 0.25; value: root.tk.price || 1; onMoved: Store.game.setTicketPrice(Math.round(value * 100)) }
                        Text { text: "Typical next home game at this price"; color: Theme.inkSubtle; font.pixelSize: 12 }
                        Text { text: (root.tk.typical || 0) + " fans of " + (root.vm.capacity || 0) + " (" + (root.tk.full || 0) + "% full)"; color: Theme.ink; font.bold: true }
                        Text { text: "Actual crowd is a roll, about " + (root.tk.lo || 0) + "–" + (root.tk.hi || 0) + ". " + (root.tk.nextPromo ? "A promo night keeps it tighter." : "No promo this week: quieter nights swing more."); color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                        Text { text: "Gate " + (root.tk.gate || "") + " + concessions " + (root.tk.concessions || "") + " at the typical crowd"; color: Theme.inkMuted; font.pixelSize: 13 }
                        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider }
                        Text { text: "If you charged… (typical crowd, not a guarantee)"; color: Theme.inkSubtle; font.pixelSize: 12 }
                        Repeater {
                            model: root.tk.proj || []
                            RowLayout { Layout.fillWidth: true
                                Text { text: "$" + modelData.price; color: Theme.ink; font.bold: modelData.current; Layout.preferredWidth: 70 }
                                Text { text: "~" + modelData.fans + " fans"; color: Theme.inkMuted; Layout.fillWidth: true }
                                Text { text: modelData.take; color: Theme.ink; font.bold: true }
                            }
                        }
                    }
                    Card {
                        visible: (root.tk.favorites || []).length > 0
                        title: "Fan favorites"
                        Repeater { model: root.tk.favorites || []
                            RowLayout { Layout.fillWidth: true
                                Text { text: "★  " + modelData.name + " · " + modelData.pos + " · " + modelData.reason + (modelData.out ? " (" + modelData.out + ")" : ""); color: Theme.gold; font.pixelSize: 13; Layout.fillWidth: true; wrapMode: Text.WordWrap }
                                Text { text: "+" + modelData.boost + "%"; color: Theme.primary; font.bold: true }
                            } }
                    }
                    Card {
                        title: "Promo nights"
                        Text { text: "Costs money up front and lifts the crowd. Sometimes it goes wrong."; color: Theme.inkSubtle; font.pixelSize: 12 }
                        Repeater { model: root.tk.promosSet || []
                            RowLayout { Layout.fillWidth: true
                                Text { text: "Week " + modelData.week + ": " + modelData.name + (modelData.played ? "  (played)" : ""); color: Theme.ink; font.pixelSize: 13; Layout.fillWidth: true }
                                PrimaryButton { visible: !modelData.played; text: "Cancel"; ghost: true; implicitHeight: 28; font.pixelSize: 12; onClicked: Store.game.cancelPromo(modelData.week) }
                            } }
                        Text { visible: (root.tk.homeWeeksLeft || 0) === 0; text: "No home games left this season."; color: Theme.inkMuted }
                        Repeater { model: root.tk.promos || []
                            ColumnLayout { Layout.fillWidth: true; spacing: 4
                                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider }
                                Text { text: modelData.name; color: Theme.ink; font.bold: true; font.pixelSize: 14 }
                                Text { text: "Cost " + modelData.costText + " · crowd x" + modelData.crowd + " · risk " + modelData.risk + "%"; color: Theme.inkSubtle; font.pixelSize: 12 }
                                Text { text: "Risk: " + modelData.riskText + (modelData.fine ? " If it goes wrong you also pay up to " + modelData.fine + " and lose some of the crowd." : ""); color: Theme.inkFaint; font.pixelSize: 11; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                                Flow { Layout.fillWidth: true; spacing: 6
                                    property var promoId: modelData.id
                                    Repeater { model: modelData.weeks
                                        Chip { text: "Wk " + modelData.week; selected: modelData.on; enabled: !modelData.locked; implicitHeight: 28; font.pixelSize: 12
                                            onClicked: { if (modelData.on) Store.game.cancelPromo(modelData.week); else Store.game.schedulePromo(modelData.week, parent.parent.promoId); } } } }
                            } }
                    }
                }
            }

            // ---------------- sponsors
            ScrollView {
                id: spSv; contentWidth: availableWidth; clip: true
                ColumnLayout {
                    width: spSv.availableWidth - 8; spacing: 10
                    Card {
                        title: "Your sponsors (" + (root.sp.list ? root.sp.list.length : 0) + " of " + (root.sp.slots || 0) + ")"
                        Text { visible: root.sp.nextSlot !== null && root.sp.nextSlot !== undefined; text: "Fan interest " + root.sp.fanInterest + ". Slot " + ((root.sp.slots || 0) + 1) + " opens at " + root.sp.nextSlot + ". Winning builds it."; color: Theme.inkSubtle; font.pixelSize: 12 }
                        Text { visible: (root.sp.list || []).length === 0; text: "No sponsors. The scoreboard is just a scoreboard."; color: Theme.inkMuted }
                        Repeater { model: root.sp.list || []
                            Rectangle {
                                Layout.fillWidth: true; implicitHeight: sc.implicitHeight + 20; radius: Theme.radiusChip + 2; color: "transparent"; border.color: Theme.divider
                                RowLayout { id: sc; anchors.fill: parent; anchors.margins: 10
                                    ColumnLayout { Layout.fillWidth: true; spacing: 2
                                        Text { text: modelData.name; color: Theme.ink; font.bold: true; font.pixelSize: 15 }
                                        Text { text: modelData.perGame + "/game · " + modelData.seasons + " season(s) left"; color: Theme.inkSubtle; font.pixelSize: 12 }
                                        Text { visible: modelData.clause !== ""; text: modelData.clause; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                                        Pill { text: modelData.effect.text; fg: root.tone(modelData.effect.tone === "neutral" ? "warn" : modelData.effect.tone)[0]; bg: modelData.effect.tone === "neutral" ? Theme.surfaceAlt : root.tone(modelData.effect.tone)[1]; implicitHeight: 22 }
                                    }
                                    PrimaryButton { text: "Cancel"; ghost: true; implicitHeight: 32; onClicked: Store.game.cancelSponsor(index) }
                                }
                            } }
                    }
                    Card {
                        title: "Offers"
                        Text { visible: (root.sp.offers || []).length === 0; text: "No offers right now. New ones show up during the season and each offseason."; color: Theme.inkMuted; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                        Repeater { model: root.sp.offers || []
                            Rectangle {
                                Layout.fillWidth: true; implicitHeight: oc.implicitHeight + 20; radius: Theme.radiusChip + 2; color: Theme.primaryTint; border.color: Theme.primaryDark
                                RowLayout { id: oc; anchors.fill: parent; anchors.margins: 10
                                    ColumnLayout { Layout.fillWidth: true; spacing: 2
                                        Text { text: modelData.name; color: Theme.ink; font.bold: true; font.pixelSize: 15 }
                                        Text { text: modelData.perGame + "/game · " + modelData.seasons + " season(s)" + (modelData.expires > 0 ? " · open until week " + modelData.expires : ""); color: Theme.inkSubtle; font.pixelSize: 12 }
                                        Text { text: modelData.clause; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                                        Pill { text: modelData.effect.text; fg: root.tone(modelData.effect.tone === "neutral" ? "warn" : modelData.effect.tone)[0]; bg: modelData.effect.tone === "neutral" ? Theme.surfaceAlt : root.tone(modelData.effect.tone)[1]; implicitHeight: 22 }
                                    }
                                    PrimaryButton { text: "Accept"; implicitHeight: 34; enabled: !root.sp.full; onClicked: Store.game.acceptSponsor(index) }
                                }
                            } }
                    }
                }
            }

            // ---------------- facilities
            ScrollView {
                id: fcSv; contentWidth: availableWidth; clip: true
                ColumnLayout {
                    width: fcSv.availableWidth - 8; spacing: 10
                    Card {
                        title: "Stadium"
                        Repeater { model: root.fc.list || []
                            ColumnLayout { Layout.fillWidth: true; spacing: 4
                                Rectangle { visible: index > 0; Layout.fillWidth: true; height: 1; color: Theme.divider }
                                RowLayout { Layout.fillWidth: true; spacing: 12
                                    ColumnLayout { Layout.fillWidth: true; spacing: 3
                                        Text { text: modelData.name; color: Theme.ink; font.bold: true; font.pixelSize: 15 }
                                        Row { spacing: 3
                                            Repeater { model: modelData.max; Rectangle { width: 16; height: 6; radius: 2; color: index < modelData.level ? Theme.primary : Theme.divider } }
                                            Text { text: "  Level " + modelData.level + " of " + modelData.max; color: Theme.inkSubtle; font.pixelSize: 12 }
                                        }
                                        Text { text: modelData.effect; color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                                        Text { text: "Upkeep " + modelData.upkeep + "/game per level"; color: Theme.inkFaint; font.pixelSize: 11 }
                                    }
                                    ColumnLayout { spacing: 2
                                        Text { visible: modelData.cost === null; text: "Maxed"; color: Theme.primary; font.bold: true }
                                        PrimaryButton { visible: modelData.cost !== null; text: modelData.costText || ""; implicitHeight: 38; implicitWidth: 100; enabled: modelData.canBuy; onClicked: Store.game.upgradeFacility(modelData.key) }
                                        PrimaryButton { visible: modelData.level > 0; text: "Tear down"; ghost: true; implicitHeight: 26; implicitWidth: 100; font.pixelSize: 11; onClicked: { tearDlg.fac = modelData; tearDlg.open(); } }
                                    }
                                }
                            } }
                        Text { text: "Total upkeep: " + (root.fc.upkeep || "") + " per game · Capacity " + (root.vm.capacity || 0) + " seats" + (root.fc.frozen ? "\nThe bank has frozen stadium spending until you pay your loan below " + root.fc.frozenBelow + "." : ""); color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    }
                }
            }

            // ---------------- payroll
            ScrollView {
                id: pySv; contentWidth: availableWidth; clip: true
                ColumnLayout {
                    width: pySv.availableWidth - 8; spacing: 10
                    RowLayout { Layout.fillWidth: true; spacing: 10
                        StatTile { label: "Pay per game"; value: root.pr.active || "" }
                        StatTile { label: "Reserve pay"; value: root.pr.reserve || "" }
                        StatTile { label: "Per season"; value: root.pr.season || "" }
                    }
                    Text { text: "Players who dress for games get their full pay. Reserves still get a small cut so they stay on the roster. Tap a player for his bio or to release him."; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    PrimaryButton { visible: (root.pr.rows || []).length > 12; text: root.allPay ? "Show top 12" : "Show all " + root.pr.rows.length; ghost: true; implicitHeight: 30; onClicked: root.allPay = !root.allPay }
                    Repeater {
                        model: root.allPay ? (root.pr.rows || []) : (root.pr.rows || []).slice(0, 12)
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 54; radius: Theme.radiusChip + 2; color: hvp.hovered ? Theme.surfaceAlt : Theme.surface; border.color: Theme.divider
                            HoverHandler { id: hvp; cursorShape: Qt.PointingHandCursor }
                            TapHandler { onTapped: Store.openPlayer(modelData.id) }
                            RowLayout { anchors.fill: parent; anchors.margins: 8; spacing: 10
                                Rectangle { width: 36; height: 36; radius: 9; color: Theme.surfaceAlt; Text { anchors.centerIn: parent; text: modelData.ovr; color: modelData.active ? Theme.ovrColor(modelData.ovr) : Theme.inkFaint; font.bold: true; font.pixelSize: 15 } }
                                ColumnLayout { Layout.fillWidth: true; spacing: 0
                                    Text { text: modelData.name; color: Theme.ink; font.bold: true; font.pixelSize: 14; elide: Text.ElideRight; Layout.fillWidth: true }
                                    Text { text: modelData.pos + " · " + (modelData.active ? "Active" : "Reserve") + " · " + modelData.years + (modelData.years === 1 ? " season" : " seasons") + " left"; color: Theme.inkSubtle; font.pixelSize: 12 }
                                }
                                ColumnLayout { spacing: 0
                                    Text { text: modelData.pay; color: Theme.ink; font.bold: true; font.pixelSize: 14; Layout.alignment: Qt.AlignRight }
                                    Text { text: modelData.active ? "per game" : "reserve pay"; color: Theme.inkSubtle; font.pixelSize: 11; Layout.alignment: Qt.AlignRight }
                                }
                            }
                        }
                    }
                }
            }

            // ---------------- loan
            ScrollView {
                id: lnSv; contentWidth: availableWidth; clip: true
                ColumnLayout {
                    width: lnSv.availableWidth - 8; spacing: 10
                    Card {
                        title: "Bank loan"
                        RowLayout { Layout.fillWidth: true; spacing: 10
                            StatTile { label: "Balance"; value: root.ln.balanceText || "" }
                            StatTile { label: "Credit limit"; value: root.ln.limit || "" }
                            StatTile { label: "Room left"; value: root.ln.roomText || "" }
                        }
                        Text { text: "Interest is " + (root.ln.rate || "") + "% per game right now and climbs the more of your line you use. The bank takes " + (root.ln.minPay || "") + "% of the balance as a minimum payment each game, and a losing season shrinks your credit limit."; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                        Flow { Layout.fillWidth: true; spacing: 8
                            Repeater { model: [5000, 10000, 25000]; PrimaryButton { text: "Borrow " + Theme.money(modelData); ghost: true; implicitHeight: 34; enabled: modelData <= root.ln.room; onClicked: Store.game.takeLoan(modelData) } }
                            Repeater { model: [5000, 10000]; PrimaryButton { text: "Repay " + Theme.money(modelData); ghost: true; implicitHeight: 34; enabled: root.ln.balance > 0 && root.ln.cash >= modelData; onClicked: Store.game.repayLoan(modelData) } }
                        }
                    }
                }
            }

            // ---------------- council
            ScrollView {
                id: coSv; contentWidth: availableWidth; clip: true
                ColumnLayout {
                    width: coSv.availableWidth - 8; spacing: 10
                    Card {
                        title: "Town Council goals, season " + (root.co.season || "")
                        Text { visible: (root.co.goals || []).length === 0; text: "The Council has not asked anything of you this year."; color: Theme.inkMuted }
                        Repeater { model: root.co.goals || []
                            RowLayout { Layout.fillWidth: true; spacing: 8
                                Text { text: modelData.met ? "✔" : "○"; color: modelData.met ? Theme.goodFg : Theme.inkFaint; font.pixelSize: 16 }
                                ColumnLayout { Layout.fillWidth: true; spacing: 0
                                    Text { text: modelData.label; color: Theme.ink; font.pixelSize: 14; font.bold: true }
                                    Text { text: modelData.progress; color: Theme.inkSubtle; font.pixelSize: 12 }
                                }
                                Text { text: modelData.reward; color: Theme.gold; font.bold: true }
                            } }
                        Text { text: (root.co.goals || []).length === 0 ? "Each goal met pays its grant in cash at season end." : "The amount beside each goal is paid in cash at season end if you meet it. Meet all " + root.co.goals.length + " and the Council adds a bonus: " + root.co.allBonus + " in all."; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    }
                    Card { visible: (root.co.reports || []).length === 0; title: "Report card"; Text { text: "The Council files its first review when the season ends."; color: Theme.inkMuted } }
                    Repeater { model: root.co.reports || []
                        Card {
                            title: "Season " + modelData.season + " review"
                            Text { text: modelData.record + " · " + modelData.finish; color: Theme.ink; font.bold: true }
                            Text { text: "Revenue " + modelData.revenue + " · expenses " + modelData.expenses + " · cash at end " + modelData.cashEnd; color: Theme.inkMuted; font.pixelSize: 12 }
                            Text { text: "Avg crowd " + modelData.avgCrowd + " of " + modelData.capacity + " · fan interest " + modelData.fanInterest + " · league avg revenue " + modelData.leagueAvgRevenue + ", net " + modelData.leagueAvgNet; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                            Repeater { model: modelData.goals; Text { text: (modelData.met ? "✔ " : "✘ ") + modelData.label + "  " + (modelData.met ? modelData.reward : ""); color: modelData.met ? Theme.goodFg : Theme.badFg; font.pixelSize: 13 } }
                            Text { text: "Grant paid: " + modelData.grant; color: Theme.gold; font.bold: true }
                        }
                    }
                }
            }
            SheetPanel {}
        }
    }

    Dialog {
        id: tearDlg
        property var fac: ({})
        parent: Overlay.overlay; anchors.centerIn: parent; modal: true
        title: "Tear down " + (fac.name || "") + " level " + (fac.level || 0) + "?"
        standardButtons: Dialog.Cancel | Dialog.Ok
        onAccepted: Store.game.downgradeFacility(fac.key)
        contentItem: Text { text: "You save " + (tearDlg.fac.upkeep || "") + " a game in upkeep, but get no refund. Building it back costs " + (tearDlg.fac.rebuild || "") + ".\n\nLost: " + (tearDlg.fac.effect || ""); color: Theme.inkMuted; wrapMode: Text.WordWrap; width: 340 }
        background: Rectangle { color: Theme.surface; radius: Theme.radiusCard; border.color: Theme.outline }
        header: Text { text: tearDlg.title; color: Theme.ink; font.bold: true; font.pixelSize: 16; padding: 16 }
    }
}
