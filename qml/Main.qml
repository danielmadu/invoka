import QtQuick
import QtQuick.Window

import io.invoka.launcher

Window {
    id: root

    // Theme tokens come straight from the Rust controller (colors.toml).
    readonly property color bg: controller.background
    readonly property color fg: controller.foreground
    readonly property color accent: controller.accent
    readonly property color selection: controller.selection
    readonly property color muted: controller.muted

    readonly property int rowHeight: 50
    readonly property int maxVisibleRows: 7
    readonly property int headerHeight: 64

    property var rows: []
    property int selectedIndex: 0

    function runSearch(text) {
        try {
            root.rows = JSON.parse(controller.search(text))
        } catch (e) {
            console.log("[invoka] search error: " + e)
            root.rows = []
        }
        root.selectedIndex = 0
    }

    function centerOnScreen() {
        x = Math.round((Screen.width - width) / 2)
        y = Math.round(Screen.height * 0.22)
    }

    function summon() {
        root.centerOnScreen()
        searchInput.text = ""
        root.runSearch("")
        searchInput.forceActiveFocus()
        root.raise()
        root.requestActivate()
    }

    visible: controller.visible
    width: 560
    height: headerHeight + Math.min(rows.length, maxVisibleRows) * rowHeight
    minimumHeight: headerHeight
    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint | Qt.Tool
    color: "transparent"
    title: "Invoka"

    Component.onCompleted: root.centerOnScreen()

    onVisibleChanged: {
        if (visible) {
            root.summon()
        }
    }

    onActiveChanged: {
        if (!active && visible) {
            controller.visible = false
        }
    }

    Controller {
        id: controller

        Component.onCompleted: controller.bootstrap()
    }

    Rectangle {
        id: card

        anchors.fill: parent
        radius: 12
        color: root.bg
        border.color: Qt.rgba(root.accent.r, root.accent.g, root.accent.b, 0.25)
        border.width: 1

        Item {
            id: header

            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: root.headerHeight

            Text {
                id: searchGlyph

                anchors.left: parent.left
                anchors.leftMargin: 18
                anchors.verticalCenter: parent.verticalCenter

                text: "\uE67E"
                color: root.muted
                font.family: "JetBrainsMono Nerd Font"
                font.pixelSize: 16
            }

            TextInput {
                id: searchInput

                anchors.left: searchGlyph.right
                anchors.leftMargin: 12
                anchors.right: parent.right
                anchors.rightMargin: 18
                anchors.verticalCenter: parent.verticalCenter

                color: root.fg
                font.pixelSize: 16
                font.family: "JetBrainsMono Nerd Font"
                clip: true
                focus: true
                cursorDelegate: Rectangle {
                    color: root.accent
                    visible: searchInput.cursorVisible
                    width: 2
                    radius: 1
                }

                Text {
                    anchors.fill: parent
                    verticalAlignment: Text.AlignVCenter
                    text: "Search apps..."
                    color: root.muted
                    font: searchInput.font
                    visible: searchInput.text.length === 0
                }

                onTextChanged: root.runSearch(text)

                Keys.onUpPressed: {
                    if (root.selectedIndex > 0) {
                        root.selectedIndex -= 1
                    }
                }
                Keys.onDownPressed: {
                    if (root.selectedIndex < root.rows.length - 1) {
                        root.selectedIndex += 1
                    }
                }
                Keys.onReturnPressed: controller.activateIndex(root.selectedIndex)
                Keys.onEnterPressed: controller.activateIndex(root.selectedIndex)
                Keys.onEscapePressed: controller.visible = false
            }
        }

        Rectangle {
            id: divider

            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: header.bottom
            height: 1
            color: Qt.rgba(root.fg.r, root.fg.g, root.fg.b, 0.08)
            visible: root.rows.length > 0
        }

        Flickable {
            id: listFlickable

            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: divider.bottom
            anchors.bottom: parent.bottom
            clip: true
            contentWidth: width
            contentHeight: listColumn.height
            interactive: contentHeight > height

            Column {
                id: listColumn

                anchors.left: parent.left
                anchors.right: parent.right

                Repeater {
                    model: root.rows

                    delegate: Item {
                        id: rowDelegate

                        readonly property bool isSelected: index === root.selectedIndex
                        readonly property bool isHovered: rowMouse.containsMouse
                        readonly property string rowName: typeof modelData !== "undefined" && modelData ? (modelData.name || "") : ""
                        readonly property string rowIcon: typeof modelData !== "undefined" && modelData ? (modelData.icon || "") : ""

                        width: listColumn.width
                        height: root.rowHeight

                        Rectangle {
                            id: highlight

                            anchors.fill: parent
                            anchors.margins: 4
                            radius: 8
                            color: root.selection
                            opacity: rowDelegate.isSelected ? 1 : (rowDelegate.isHovered ? 0.55 : 0)
                            Behavior on opacity {
                                NumberAnimation {
                                    duration: 60
                                }
                            }
                        }

                        Rectangle {
                            id: avatar

                            anchors.left: parent.left
                            anchors.leftMargin: 14
                            anchors.verticalCenter: parent.verticalCenter
                            width: 26
                            height: 26
                            radius: 6
                            color: Qt.rgba(root.accent.r, root.accent.g, root.accent.b, 0.16)
                            border.color: Qt.rgba(root.accent.r, root.accent.g, root.accent.b, 0.35)
                            border.width: 1

                            Image {
                                id: appIcon

                                anchors.fill: parent
                                anchors.margins: 3
                                fillMode: Image.PreserveAspectFit
                                smooth: true
                                source: rowIcon.startsWith("file:") ? rowIcon : (rowIcon.length > 0 ? ("file://" + rowIcon) : "")
                                visible: status === Image.Ready
                                asynchronous: true
                            }

                            Text {
                                anchors.centerIn: parent
                                visible: !appIcon.visible
                                text: rowName.charAt(0).toUpperCase()
                                color: root.accent
                                font.family: "JetBrainsMono Nerd Font"
                                font.pixelSize: 13
                                font.bold: true
                            }
                        }

                        Text {
                            anchors.left: avatar.right
                            anchors.leftMargin: 12
                            anchors.right: parent.right
                            anchors.rightMargin: 14
                            anchors.verticalCenter: parent.verticalCenter

                            text: rowName
                            color: root.fg
                            font.family: "JetBrainsMono Nerd Font"
                            font.pixelSize: 14
                            elide: Text.ElideRight
                        }

                        MouseArea {
                            id: rowMouse

                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor

                            onClicked: controller.activateIndex(index)
                            onPositionChanged: root.selectedIndex = index
                        }
                    }
                }

                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.topMargin: 14
                    topPadding: 14

                    text: "No matching apps"
                    color: root.muted
                    font.family: "JetBrainsMono Nerd Font"
                    font.pixelSize: 13
                    visible: root.rows.length === 0 && searchInput.text.length > 0
                }
            }
        }
    }
}
