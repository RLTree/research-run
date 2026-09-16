"""Binary-independent filesystem, budget and query-selection controls."""
import json
import pathlib
import tempfile

from measure import (corpus_is_expected, derive_queries, digest_state, expected_record,
                     select_knowledge_files)
from measurement_fs import (MAX_INVENTORY_RECORD_BYTES, MAX_RECORD_BYTES,
                            ReadBudget, read_regular, safe_files)


def run_controls():
    with tempfile.TemporaryDirectory() as temporary:
        workspace = pathlib.Path(temporary) / 'corpus-workspace'
        knowledge = workspace / '.research-run/knowledge'
        knowledge.mkdir(parents=True)
        measurement_files = workspace / '.research-run/measurements'
        measurement_files.mkdir()
        budget = workspace / '.research-run/inventories'
        budget.mkdir()
        exact = measurement_files / 'exact.bin'
        exact.touch()
        exact.write_bytes(b'')
        with exact.open('r+b') as stream:
            stream.truncate(MAX_RECORD_BYTES)
        read_regular(exact, workspace)
        oversized = measurement_files / 'oversized.bin'
        oversized.touch()
        with oversized.open('r+b') as stream:
            stream.truncate(MAX_RECORD_BYTES + 1)
        try:
            read_regular(oversized, workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('oversized regular file was accepted')
        oversized.unlink()
        aggregate_a = measurement_files / 'aggregate-a.bin'
        aggregate_b = measurement_files / 'aggregate-b.bin'
        for path in (aggregate_a, aggregate_b):
            path.touch()
            with path.open('r+b') as stream:
                stream.truncate(MAX_RECORD_BYTES // 2)
        aggregate_budget = ReadBudget(MAX_RECORD_BYTES)
        read_regular(aggregate_a, workspace, aggregate_budget)
        read_regular(aggregate_b, workspace, aggregate_budget)
        overflow_a = measurement_files / 'overflow-a.bin'
        overflow_b = measurement_files / 'overflow-b.bin'
        for path in (overflow_a, overflow_b):
            path.touch()
            with path.open('r+b') as stream:
                stream.truncate((MAX_RECORD_BYTES * 3) // 5)
        overflow_budget = ReadBudget(MAX_RECORD_BYTES)
        read_regular(overflow_a, workspace, overflow_budget)
        try:
            read_regular(overflow_b, workspace, overflow_budget)
        except RuntimeError:
            pass
        else:
            raise AssertionError('aggregate over-budget files were accepted')
        inventory_exact = budget / 'exact.bin'
        inventory_exact.touch()
        with inventory_exact.open('r+b') as stream:
            stream.truncate(MAX_INVENTORY_RECORD_BYTES)
        read_regular(inventory_exact, workspace)
        inventory_over = budget / 'oversized.bin'
        inventory_over.touch()
        with inventory_over.open('r+b') as stream:
            stream.truncate(MAX_INVENTORY_RECORD_BYTES + 1)
        try:
            read_regular(inventory_over, workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('oversized inventory file was accepted')
        inventory_over.unlink()
        for index in range(2):
            (knowledge / f'note-{index:04}.json').write_bytes(expected_record(index, 200))
        if not corpus_is_expected(workspace, 2, 200):
            raise AssertionError('complete corpus was rejected')
        (knowledge / 'note-0001.json').unlink()
        if corpus_is_expected(workspace, 2, 200):
            raise AssertionError('missing record was accepted')
        (knowledge / 'note-0001.json').write_bytes(expected_record(1, 200).replace(b'tail-needle', b'changed-text'))
        if corpus_is_expected(workspace, 2, 200):
            raise AssertionError('changed record was accepted')
        (knowledge / 'note-0001.json').write_bytes(expected_record(1, 200))
        (knowledge / 'note-0002.json').write_bytes(expected_record(2, 200))
        if corpus_is_expected(workspace, 2, 200):
            raise AssertionError('extra record was accepted')
        (knowledge / 'note-0002.json').unlink()
        if not corpus_is_expected(workspace, 2, 200):
            raise AssertionError('restored corpus was rejected')
        short_workspace = workspace.parent / 'short-workspace'
        short_knowledge = short_workspace / '.research-run/knowledge'
        short_knowledge.mkdir(parents=True)
        short_record = short_knowledge / 'short.json'
        short_record.write_text(json.dumps({'body': 'short body'}))
        try:
            derive_queries(short_workspace, [short_record])
        except RuntimeError:
            pass
        else:
            raise AssertionError('short body was accepted for tail measurement')
        query_workspace = workspace.parent / 'query-workspace'
        query_knowledge = query_workspace / '.research-run/knowledge'
        query_knowledge.mkdir(parents=True)
        direct = query_knowledge / 'direct.json'
        direct.write_text(json.dumps({'body': 'x' * 600 + ' direct-tail'}))
        nested = query_workspace / '.research-run/nested/knowledge'
        nested.mkdir(parents=True)
        nested_record = nested / 'nested.json'
        nested_record.write_text(json.dumps({'body': 'x' * 600 + ' nested-tail'}))
        selected = select_knowledge_files(query_workspace, safe_files(query_workspace))
        if direct not in selected or nested_record in selected:
            raise AssertionError('nested knowledge file was selected')
        derive_queries(query_workspace, selected)
        (knowledge / 'linked').symlink_to(knowledge / 'note-0000.json')
        try:
            safe_files(workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('symlink file was accepted')
        (knowledge / 'linked').unlink()
        outside_directory = workspace / 'outside-directory'
        outside_directory.mkdir()
        (knowledge / 'linked-directory').symlink_to(outside_directory, target_is_directory=True)
        try:
            safe_files(workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('symlink directory was accepted')
        (knowledge / 'linked-directory').unlink()
        linked_workspace = workspace.parent / f'linked-workspace-{workspace.name}'
        if linked_workspace.exists() or linked_workspace.is_symlink():
            linked_workspace.unlink()
        linked_workspace.symlink_to(workspace, target_is_directory=True)
        try:
            safe_files(linked_workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('symlink workspace was accepted')
        linked_workspace.unlink()
        actual_parent = workspace.parent / f'actual-parent-{workspace.name}'
        actual_workspace = actual_parent / 'nested'
        (actual_workspace / '.research-run').mkdir(parents=True)
        linked_parent = workspace.parent / f'linked-parent-{workspace.name}'
        linked_parent.symlink_to(actual_parent, target_is_directory=True)
        try:
            safe_files(linked_parent / 'nested')
        except RuntimeError:
            pass
        else:
            raise AssertionError('symlink ancestor was accepted')
        linked_parent.unlink()
        (actual_workspace / '.research-run').rmdir()
        actual_workspace.rmdir()
        actual_parent.rmdir()
        linked_state_workspace = workspace.parent / f'linked-state-{workspace.name}'
        linked_state_workspace.mkdir()
        (linked_state_workspace / '.research-run').symlink_to(knowledge, target_is_directory=True)
        try:
            safe_files(linked_state_workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('symlink research state was accepted')
        (linked_state_workspace / '.research-run').unlink()
        linked_state_workspace.rmdir()
        (knowledge / 'fifo').parent.mkdir(exist_ok=True)
        import os
        os.mkfifo(knowledge / 'fifo')
        try:
            safe_files(workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('FIFO was accepted')
    print('measurement helper controls passed')

