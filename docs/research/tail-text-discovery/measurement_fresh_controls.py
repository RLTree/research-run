"""Exercise real self-test entry points in a disposable checkout without binaries."""
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile


def run_controls():
    source = pathlib.Path(__file__).parent
    names = ('measure.py', 'measurement_fs.py', 'measurement_fixture.py',
             'measurement_fixture_controls.py', 'measurement_read_controls.py')
    with tempfile.TemporaryDirectory() as temporary:
        checkout = pathlib.Path(temporary).resolve() / 'checkout'
        scripts = checkout / 'docs/research/tail-text-discovery'
        scripts.mkdir(parents=True)
        for name in names:
            shutil.copyfile(source / name, scripts / name)
        environment = dict(os.environ, PYTHONDONTWRITEBYTECODE='1')
        for flags in ([], ['-O']):
            for action in ('self-test', 'entry-test'):
                result = subprocess.run(
                    [sys.executable, '-B', *flags, str(scripts / 'measure.py'), action],
                    cwd=checkout, env=environment, capture_output=True, timeout=30)
                if result.returncode != 0:
                    raise RuntimeError(f'fresh {action} failed: {result.stderr.decode()}')
                if (checkout / 'target').exists():
                    raise RuntimeError('binary-independent controls created evidence parents')
                print(f'fresh {action}, optimized={bool(flags)}: PASS; no target or binaries')


if __name__ == '__main__':
    run_controls()
