// SPDX-License-Identifier: GPL-2.0-or-later
#include <QQmlEngine>
#include <QQmlExtensionPlugin>
#include <QVariantMap>
#include <cmath>
#include <kwin/main.h>
#include <kwin/inputmethod.h>

// This module is loaded by a temporary KWin QML script, not by Vispeak.
// It reads compositor text-input geometry without changing the input method.
class CaretReader final : public QObject
{
    Q_OBJECT
public:
    Q_INVOKABLE QVariantMap read(QObject *activeWindow) const
    {
        const auto unavailable = [](const char *reason) {
            return QVariantMap{{"available", false}, {"reason", QString::fromLatin1(reason)}};
        };
        // KWin's internal C++ API requires a build for the exact compositor.
        if (QCoreApplication::applicationVersion() != KWIN_VERSION_STRING)
            return unavailable("KWin version mismatch");
        auto *application = qobject_cast<KWin::Application *>(QCoreApplication::instance());
        if (!application || !activeWindow)
            return unavailable("No active KWin window");
        auto *input = application->inputMethod();
        if (!input || !input->isActive() || !input->activeClientSupportsTextInput())
            return unavailable("Wayland text-input is inactive");
        // KWin::Window directly inherits QObject. Comparing addresses avoids
        // querying a stale text-input surface belonging to a background window.
        if (static_cast<void *>(input->activeWindow()) != static_cast<void *>(activeWindow))
            return unavailable("Text-input belongs to another window");
        const auto rect = input->cursorRectangle();
        if (!std::isfinite(rect.x()) || !std::isfinite(rect.y())
            || !std::isfinite(rect.width()) || !std::isfinite(rect.height())
            || rect.width() < 0 || rect.height() <= 0)
            return unavailable("Compositor returned invalid caret geometry");
        return {{"available", true}, {"x", rect.x()}, {"y", rect.y()},
                {"width", rect.width()}, {"height", rect.height()}};
    }
};

class CaretBridgePlugin final : public QQmlExtensionPlugin
{
    Q_OBJECT
    Q_PLUGIN_METADATA(IID "org.qt-project.Qt.QQmlExtensionInterface/1.0")
public:
    void registerTypes(const char *uri) override
    {
        qmlRegisterSingletonType<CaretReader>(uri, 1, 0, "CaretReader",
            [](QQmlEngine *, QJSEngine *) -> QObject * { return new CaretReader; });
    }
};

#include "bridge.moc"
