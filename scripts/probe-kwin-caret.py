#!/usr/bin/env python3
"""Read KWin text-input geometry through a temporary native QML bridge.

Run only after approval to load the reviewed module into the compositor.
This does not enable a persistent KWin script or change input-method settings.
"""
import json
from pathlib import Path
import sys
import tempfile

from gi.repository import Gio, GLib


def main():
    module = Path(sys.argv[1]).resolve()
    if not (module / "qmldir").is_file() or not (module / "libVispeakCaretBridge.so").is_file():
        raise SystemExit("QML bridge module is missing")
    connection = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    loop = GLib.MainLoop()
    received = []
    errors = []

    def call(path, interface, method, args):
        return connection.call_sync("org.kde.KWin", path, interface, method, args,
                                    None, Gio.DBusCallFlags.NONE, 3000, None).unpack()

    owner = connection.call_sync("org.freedesktop.DBus", "/org/freedesktop/DBus",
        "org.freedesktop.DBus", "GetNameOwner", GLib.Variant("(s)", ("org.kde.KWin",)),
        None, Gio.DBusCallFlags.NONE, 3000, None).unpack()[0]

    def receive(conn, sender, path, interface, method, parameters, invocation):
        if sender != owner:
            invocation.return_dbus_error("app.vispeak.UntrustedSender", "Expected KWin")
            return
        received.append(json.loads(parameters.unpack()[0]))
        invocation.return_value(GLib.Variant("()", ()))
        loop.quit()

    xml = '<node><interface name="app.vispeak.CaretProbe"><method name="Report"><arg name="data" type="s" direction="in"/></method></interface></node>'
    connection.register_object("/app/vispeak/CaretProbe",
        Gio.DBusNodeInfo.new_for_xml(xml).interfaces[0], receive, None, None)
    scratch = Path(__file__).resolve().parents[1] / "src-tauri" / "target"
    scratch.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vispeak-caret-probe-", dir=scratch) as directory:
        name = Path(directory).name
        path = Path(directory) / "probe.qml"
        path.write_text(f'''import QtQuick
import org.kde.kwin
import {json.dumps(module.as_uri())} as Bridge
DBusCall {{
    id: probe
    service: {json.dumps(connection.get_unique_name())}
    path: "/app/vispeak/CaretProbe"
    dbusInterface: "app.vispeak.CaretProbe"
    method: "Report"
    Component.onCompleted: {{
        var window = Workspace.activeWindow;
        probe.arguments = [JSON.stringify({{pid: window ? window.pid : 0,
            caret: Bridge.CaretReader.read(window)}})];
        probe.call();
    }}
}}
''')
        loaded = False
        try:
            index = call("/Scripting", "org.kde.kwin.Scripting", "loadDeclarativeScript",
                         GLib.Variant("(ss)", (str(path), name)))[0]
            if index < 0:
                raise RuntimeError("KWin refused the temporary script")
            loaded = True
            def started(conn, result, data):
                try:
                    conn.call_finish(result)
                except GLib.Error as error:
                    errors.append(str(error))
                    loop.quit()

            connection.call("org.kde.KWin", f"/Scripting/Script{index}",
                "org.kde.kwin.Script", "run", None, None, Gio.DBusCallFlags.NONE,
                3000, None, started, None)
            GLib.timeout_add(3000, lambda: loop.quit() or False)
            loop.run()
            if not received:
                raise RuntimeError(errors[0] if errors else "KWin did not return caret geometry")
            print(json.dumps(received[0]))
        finally:
            if loaded:
                call("/Scripting", "org.kde.kwin.Scripting", "unloadScript",
                     GLib.Variant("(s)", (name,)))


if __name__ == "__main__":
    main()
