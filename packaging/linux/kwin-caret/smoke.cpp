// SPDX-License-Identifier: GPL-2.0-or-later
#include <QCoreApplication>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QVariantMap>
#include <QDebug>

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    if (argc != 2)
        return 2;
    QQmlEngine engine;
    engine.addImportPath(QString::fromLocal8Bit(argv[1]));
    QQmlComponent component(&engine);
    component.setData("import QtQml\nimport Vispeak.CaretBridge 1.0\nQtObject { property var snapshot: CaretReader.read(null) }", QUrl());
    auto *object = component.create();
    if (!object) {
        qWarning() << component.errors();
        return 1;
    }
    const auto snapshot = object->property("snapshot").toMap();
    delete object;
    if (snapshot.value("available", true).toBool())
        return 1;
    qInfo() << "Plugin loads and rejects a non-KWin process without querying compositor APIs";
    return 0;
}
