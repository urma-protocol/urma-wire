#!/usr/bin/env python3
"""Bounded Linux desktop smoke on a disposable profile and private Xvfb/DBus.

Run after compiling the application. No user vault or live chain source is used.
The missing-vault session response exercises native IPC; this is not a signing,
chain, publication, or exhaustive CSP/security test. Screenshots need visual review.
"""
import argparse
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import tempfile
import time


def run(args, **kwargs):
    return subprocess.run(args, check=True, timeout=10, **kwargs)


def child(args):
    import gi
    gi.require_version('Atspi', '2.0')
    from gi.repository import Atspi, GLib

    evidence = Path(args.evidence)
    processes = []
    checks = []
    log = (evidence / 'application.log').open('w')
    xlog = (evidence / 'xvfb.log').open('w')
    read_fd, write_fd = os.pipe()
    try:
        processes.append(subprocess.Popen(
            ['Xvfb', '-displayfd', str(write_fd), '-screen', '0', '1280x1000x24',
             '-nolisten', 'tcp'], pass_fds=(write_fd,), stdout=xlog, stderr=xlog))
        if not select.select([read_fd], [], [], 8)[0]:
            raise AssertionError('Xvfb allocation timeout')
        display = os.read(read_fd, 64).decode().strip()
        assert display.isdigit(), display
        os.environ['DISPLAY'] = ':' + display
        os.environ['ATSPI_DBUS_IMPLEMENTATION'] = 'dbus-daemon'
        run(['dbus-update-activation-environment', 'DISPLAY', 'ATSPI_DBUS_IMPLEMENTATION'])
        Atspi.set_timeout(500, 1500)
        Atspi.init()
        Atspi.get_desktop(0)
        command = [args.binary] if args.binary else ['cargo', 'tauri', 'dev', '--no-watch']
        app = subprocess.Popen(command, cwd=args.repo, stdout=log, stderr=log)
        processes.append(app)

        def tree():
            rows, queue = [], []
            desktop = Atspi.get_desktop(0)
            for index in range(desktop.get_child_count()):
                node = desktop.get_child_at_index(index)
                try:
                    if node and os.getpgid(node.get_process_id()) == os.getpgrp():
                        queue.append((node, 0))
                except (ProcessLookupError, GLib.Error):
                    continue
            while queue and len(rows) < 2000:
                node, depth = queue.pop(0)
                try:
                    states = node.get_state_set()
                    if states.contains(Atspi.StateType.SHOWING):
                        rows.append((node, node.get_name() or '', node.get_role_name()))
                    if depth < 24:
                        for index in range(node.get_child_count()):
                            item = node.get_child_at_index(index)
                            if item:
                                queue.append((item, depth + 1))
                except GLib.Error:
                    continue
            return rows

        def wait(predicate, message, seconds=15):
            until = time.monotonic() + seconds
            while time.monotonic() < until:
                if app.poll() is not None:
                    raise AssertionError(f'application exited {app.returncode}: {message}')
                value = predicate()
                if value:
                    return value
                time.sleep(.1)
            raise AssertionError('timeout: ' + message)

        def text(expected, seconds=15, scroll=False):
            def found():
                for node, name, _ in tree():
                    if expected in name:
                        return True
                    interface = node.get_text_iface()
                    if interface:
                        try:
                            if expected in Atspi.Text.get_text(interface, 0, -1):
                                return True
                        except GLib.Error:
                            pass
                if scroll:
                    run(['xdotool', 'mousemove', '850', '650', 'click', '5'])
                return False
            wait(found, expected, seconds)
            checks.append(expected)

        def click(name):
            node = wait(lambda: next((node for node, value, role in tree()
                                      if value == name and role in ('link', 'button')), None),
                        'navigation ' + name)
            extent = node.get_component_iface().get_extents(Atspi.CoordType.SCREEN)
            assert extent.width > 0 and extent.height > 0
            run(['xdotool', 'mousemove', str(extent.x + extent.width // 2),
                 str(extent.y + extent.height // 2), 'click', '1'])

        text('Identity', 600)
        if not args.binary:
            import urllib.request
            import re
            with urllib.request.urlopen('http://127.0.0.1:8080/', timeout=5) as response:
                policy = response.headers.get('Content-Security-Policy', '')
                html = response.read().decode()
            nonce = re.search(r"'nonce-([^']+)'", policy)
            assert nonce and 'nonce=\"' + nonce[1] + '\"' in html, policy
            assert "object-src 'none'" in policy
            (evidence / 'dev-csp.json').write_text(json.dumps({'header': policy, 'nonce_matches_html': True}, indent=2))
            checks.append('served CSP nonce matches bootstrap HTML')
        click('Identity')
        text('Import an encrypted URMA vault to begin.')
        text('Locked')
        text('Refresh session status')
        click('Refresh session status')
        text('Import an encrypted URMA vault to begin.')
        run(['import', '-window', 'root', str(evidence / 'identity.png')])
        click('Settings')
        text('Your settings.')
        text('Local node', scroll=True)
        run(['import', '-window', 'root', str(evidence / 'settings.png')])
        (evidence / 'accessible.json').write_text(json.dumps(
            [{'name': name, 'role': role} for _, name, role in tree()], indent=2))
        (evidence / 'result.json').write_text(json.dumps({
            'status': 'PASS', 'checks': checks, 'command': command,
            'profile': 'disposable; missing vault; loopback port 9 local source',
            'scope': 'native IPC and visible navigation, screenshots require review',
        }, indent=2) + '\n')
        print('PASS: desktop IPC/session/navigation smoke', flush=True)
    except Exception as error:
        (evidence / 'failure.json').write_text(json.dumps({'error': str(error), 'passed': checks}, indent=2))
        if 'tree' in locals():
            (evidence / 'accessible.json').write_text(json.dumps(
                [{'name': name, 'role': role} for _, name, role in tree()], indent=2))
        if 'DISPLAY' in os.environ:
            subprocess.run(['import', '-window', 'root', str(evidence / 'failure.png')], timeout=10)
        raise
    finally:
        os.close(read_fd)
        os.close(write_fd)
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)
        log.close()
        xlog.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', default=str(Path(__file__).resolve().parents[1]))
    parser.add_argument('--evidence', required=True)
    parser.add_argument('--binary', help='test already-built packaged executable instead of cargo tauri dev')
    parser.add_argument('--child', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    args.repo = str(Path(args.repo).resolve())
    args.evidence = str(Path(args.evidence).resolve())
    if args.binary:
        args.binary = str(Path(args.binary).resolve())
    if args.child:
        child(args)
        return
    evidence = Path(args.evidence)
    evidence.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='wire-desktop-smoke-') as directory:
        work = Path(directory)
        env = {key: os.environ[key] for key in
               ('PATH', 'CARGO_HOME', 'RUSTUP_HOME', 'CARGO_TARGET_DIR', 'CARGO_BUILD_JOBS')
               if key in os.environ}
        # Keep toolchain caches explicit while isolating application HOME/XDG.
        env.setdefault('CARGO_HOME', str(Path.home() / '.cargo'))
        env.setdefault('RUSTUP_HOME', str(Path.home() / '.rustup'))
        for key, name in [('HOME', 'home'), ('XDG_CONFIG_HOME', 'config'),
                          ('XDG_DATA_HOME', 'data'), ('XDG_CACHE_HOME', 'cache'),
                          ('XDG_RUNTIME_DIR', 'runtime')]:
            target = work / name
            target.mkdir(mode=0o700)
            env[key] = str(target)
        cookie = work / 'cookie'
        cookie.write_text('fixture:fixture')
        cookie.chmod(0o600)
        config = work / 'config.json'
        config.write_text(json.dumps({'rpc_url': 'http://127.0.0.1:9',
            'node_auth_file': str(cookie), 'vault': str(work / 'missing.urma'),
            'unlock_file': str(work / 'missing-unlock')}))
        env.update(URMA_CONFIG=str(config), LANG='C.UTF-8', NO_COLOR='1',
                   GTK_A11Y='always', GTK_MODULES='gail:atk-bridge',
                   GIO_USE_VFS='local', GSETTINGS_BACKEND='memory', GDK_BACKEND='x11',
                   ATSPI_DBUS_IMPLEMENTATION='dbus-daemon', CARGO_NET_OFFLINE='true',
                   TRUNK_SKIP_VERSION_CHECK='true', TRUNK_OFFLINE='true')
        command = ['dbus-run-session', '--', sys.executable, str(Path(__file__).resolve()),
                   '--child', '--repo', args.repo, '--evidence', args.evidence]
        if args.binary:
            command += ['--binary', args.binary]
        process = subprocess.Popen(command, env=env, start_new_session=True)
        try:
            code = process.wait(timeout=660)
            for index, native_log in enumerate((work / 'data').rglob('wire.log')):
                (evidence / f'native-{index}.log').write_bytes(native_log.read_bytes())
            assert code == 0, f'desktop smoke failed: exit {code}; see {evidence}'
        finally:
            for sig in (signal.SIGTERM, signal.SIGKILL):
                try:
                    os.killpg(process.pid, sig)
                except ProcessLookupError:
                    pass
                if sig == signal.SIGTERM:
                    time.sleep(.3)
            process.wait(timeout=5)
            rows = subprocess.check_output(['ps', '-eo', 'pgid=,pid=,stat=,comm='], text=True)
            remaining = [line.strip() for line in rows.splitlines()
                         if line.split() and line.split()[0] == str(process.pid)
                         and not line.split()[2].startswith('Z')]
            (evidence / 'cleanup.json').write_text(json.dumps(
                {'owned_pgid': process.pid, 'remaining_live_processes': remaining}, indent=2))
            assert not remaining, remaining


if __name__ == '__main__':
    main()
