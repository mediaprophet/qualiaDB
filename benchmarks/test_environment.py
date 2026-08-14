"""Harness environment merge must refresh engine_version from this tree."""
from environment import merge_execution_environment, read_workspace_engine_version


def test_merge_replaces_stale_engine_version():
    workspace = read_workspace_engine_version()
    assert workspace == "0.0.30"
    merged = merge_execution_environment(
        {
            "runner": "old",
            "engine_version": "0.0.18",
            "measurement_path": "harness_isolated_subprocess",
        }
    )
    assert merged["engine_version"] == workspace


if __name__ == "__main__":
    test_merge_replaces_stale_engine_version()
    print("environment merge refreshes engine_version")
