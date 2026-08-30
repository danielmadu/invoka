#include "layershell.h"

#include <QGuiApplication>
#include <QWindow>

#include <cstdio>

#ifdef INVOKA_LAYER_SHELL

#include <LayerShellQt/Window>

namespace {

void configure(LayerShellQt::Window* shell) {
    // Anchor to all edges with a fixed window size: the compositor centers
    // the surface on both axes (standard launcher trick).
    LayerShellQt::Window::Anchors anchors;
    anchors.left = true;
    anchors.right = true;
    anchors.top = true;
    anchors.bottom = true;
    shell->setAnchors(anchors);

    // Float above regular windows; take space from no other layer.
    shell->setLayer(LayerShellQt::Window::LayerOverlay);
    shell->setExclusiveZone(-1);
    shell->setScope(QStringLiteral("launcher"));
    shell->setKeyboardInteractivity(LayerShellQt::Window::KeyboardInteractivityOnDemand);
}

}  // namespace

int invoka_layershell_setup() {
    int attached = 0;
    for (QWindow* window : QGuiApplication::topLevelWindows()) {
        // QML windows are created hidden; layer surface configuration must
        // happen before the platform window exists, so hidden windows are
        // exactly the ones we want.
        if (window->platformName() != QLatin1String("wayland")) {
            continue;
        }
        if (LayerShellQt::Window* shell = LayerShellQt::Window::attach(window)) {
            configure(shell);
            attached++;
        }
    }

    fprintf(stderr, "[invoka] layer-shell attached to %d window(s)\n", attached);
    return attached;
}

#else  // !INVOKA_LAYER_SHELL

int invoka_layershell_setup() {
    return 0;
}

#endif  // INVOKA_LAYER_SHELL
