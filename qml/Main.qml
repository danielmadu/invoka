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
    readonly property int settingsRowHeight: 44

    property var rows: []
    property int selectedIndex: 0
    property var themes: []

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

    function loadThemes() {
        try {
            root.themes = JSON.parse(controller.themes_json)
        } catch (e) {
            console.log("[invoka] themes json error: " + e)
            root.themes = []
        }
    }

    function centerSettings() {
        settings.x = Math.round((Screen.width - settings.width) / 2)
        settings.y = Math.round(Screen.height * 0.3)
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

        Component.onCompleted: {
            controller.bootstrap()
            root.loadThemes()
        }
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

    Window {
        id: settings

        visible: controller.settings_visible
        flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint | Qt.Tool
        color: "transparent"
        title: "Invoka Settings"
        width: 420
        height: settingsHeader.height
                + Math.min(root.themes.length, 7) * root.settingsRowHeight
                + customNote.height + 24

        onVisibleChanged: {
            if (visible) {
                settings.centerOnScreen()
                settings.requestActivate()
            }
        }

        onActiveChanged: {
            if (!active && visible) {
                controller.settings_visible = false
            }
        }

        function centerOnScreen() {
            x = Math.round((Screen.width - width) / 2)
            y = Math.round(Screen.height * 0.3)
        }

        Shortcut {
            sequence: "Esc"
            enabled: settings.visible
            onActivated: controller.settings_visible = false
        }

        Rectangle {
            id: settingsCard

            anchors.fill: parent
            radius: 12
            color: root.bg
            border.color: Qt.rgba(root.accent.r, root.accent.g, root.accent.b, 0.25)
            border.width: 1

            Item {
                id: settingsHeader

                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: 48

                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 18
                    anchors.verticalCenter: parent.verticalCenter

                    text: "Settings"
                    color: root.fg
                    font.family: "JetBrainsMono Nerd Font"
                    font.pixelSize: 15
                    font.bold: true
                }

                Text {
                    id: settingsClose

                    anchors.right: parent.right
                    anchors.rightMargin: 18
                    anchors.verticalCenter: parent.verticalCenter

                    text: "\u2715"
                    color: mouseOverClose.containsMouse ? root.accent : root.muted
                    font.family: "JetBrainsMono Nerd Font"
                    font.pixelSize: 14

                    MouseArea {
                        id: mouseOverClose

                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: controller.settings_visible = false
                    }
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 1
                    color: Qt.rgba(root.fg.r, root.fg.g, root.fg.b, 0.08)
                }
            }

            Flickable {
                id: settingsFlickable

                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: settingsHeader.bottom
                anchors.bottom: customNote.top
                clip: true
                contentWidth: width
                contentHeight: settingsColumn.height
                interactive: contentHeight > height

                Column {
                    id: settingsColumn

                    anchors.left: parent.left
                    anchors.right: parent.right

                    Repeater {
                        model: root.themes

                        delegate: Item {
                            id: themeRow

                            readonly property bool isActive: controller.active_theme_id === modelData.id
                            readonly property bool isHovered: themeMouse.containsMouse

                            width: settingsColumn.width
                            height: root.settingsRowHeight

                            Rectangle {
                                anchors.fill: parent
                                anchors.margins: 3
                                radius: 8
                                color: root.selection
                                opacity: themeRow.isActive ? 0.8 : (themeRow.isHovered ? 0.45 : 0)
                                Behavior on opacity {
                                    NumberAnimation {
                                        duration: 60
                                    }
                                }
                            }

                            Row {
                                anchors.left: parent.left
                                anchors.leftMargin: 16
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 6

                                Rectangle {
                                    width: 18
                                    height: 18
                                    radius: 5
                                    color: modelData.background
                                    border.color: Qt.rgba(root.fg.r, root.fg.g, root.fg.b, 0.2)
                                    border.width: 1
                                }
                                Rectangle {
                                    width: 18
                                    height: 18
                                    radius: 5
                                    color: modelData.accent
                                    border.color: Qt.rgba(root.fg.r, root.fg.g, root.fg.b, 0.2)
                                    border.width: 1
                                }
                                Rectangle {
                                    width: 18
                                    height: 18
                                    radius: 5
                                    color: modelData.foreground
                                    border.color: Qt.rgba(root.fg.r, root.fg.g, root.fg.b, 0.2)
                                    border.width: 1
                                }
                            }

                            Text {
                                anchors.left: parent.left
                                anchors.leftMargin: 88
                                anchors.right: activeMark.left
                                anchors.rightMargin: 8
                                anchors.verticalCenter: parent.verticalCenter

                                text: modelData.label
                                color: themeRow.isActive ? root.accent : root.fg
                                font.family: "JetBrainsMono Nerd Font"
                                font.pixelSize: 14
                                elide: Text.ElideRight
                            }

                            Text {
                                id: activeMark

                                anchors.right: parent.right
                                anchors.rightMargin: 16
                                anchors.verticalCenter: parent.verticalCenter

                                text: "\u2713"
                                color: root.accent
                                font.family: "JetBrainsMono Nerd Font"
                                font.pixelSize: 14
                                font.bold: true
                                visible: themeRow.isActive
                            }

                            MouseArea {
                                id: themeMouse

                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: controller.selectTheme(modelData.id)
                            }
                        }
                    }
                }
            }

            Text {
                id: customNote

                anchors.left: parent.left
                anchors.leftMargin: 18
                anchors.right: parent.right
                anchors.rightMargin: 18
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 8

                text: "\u2022 custom theme (theme.toml edited manually)"
                color: root.muted
                font.family: "JetBrainsMono Nerd Font"
                font.pixelSize: 11
                visible: controller.active_theme_id === "custom"
            }
        }
    }
}
