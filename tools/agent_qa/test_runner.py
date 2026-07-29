import json
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import runner


class ClassificationTests(unittest.TestCase):
    def test_passed(self):
        self.assertEqual(runner.classify(0, False, False, ""), "passed")

    def test_timeout_has_priority(self):
        self.assertEqual(runner.classify(1, True, False, "error[E0000]"), "timeout")

    def test_budget_failure_has_priority_over_command_output(self):
        self.assertEqual(
            runner.classify(1, False, True, "could not compile"),
            "artifact-budget-exceeded",
        )

    def test_compile_failure(self):
        self.assertEqual(
            runner.classify(101, False, False, "error[E0308]: mismatched types"),
            "compile-failure",
        )


class ManifestTests(unittest.TestCase):
    def test_scenarios_are_shell_free_command_arrays(self):
        path = pathlib.Path(__file__).with_name("scenarios.json")
        manifest = json.loads(path.read_text("utf-8"))
        for scenario in manifest["scenarios"].values():
            for step in scenario["steps"]:
                self.assertIsInstance(step["command"], list)
                self.assertGreater(len(step["command"]), 0)
                self.assertNotIn("&&", step["command"])
                self.assertNotIn("|", step["command"])
                self.assertGreater(step["timeout_seconds"], 0)

    def test_missing_command_is_structured(self):
        with tempfile.TemporaryDirectory() as directory:
            result, used = runner.run_step(
                {
                    "id": "missing",
                    "command": ["webizen-command-that-does-not-exist"],
                    "timeout_seconds": 1,
                },
                pathlib.Path(__file__).resolve().parents[2],
                pathlib.Path(directory),
                1024,
            )
        self.assertEqual(result["classification"], "command-start-failure")
        self.assertFalse(result["passed"])
        self.assertGreater(used, 0)

    def test_failure_classification_uses_output_tail(self):
        with tempfile.TemporaryDirectory() as directory:
            result, _ = runner.run_step(
                {
                    "id": "tail-error",
                    "command": [
                        sys.executable,
                        "-c",
                        "print('x' * 17000); print('error[E0308]: mismatched types'); raise SystemExit(101)",
                    ],
                    "timeout_seconds": 5,
                },
                pathlib.Path(__file__).resolve().parents[2],
                pathlib.Path(directory),
                32_000,
            )
        self.assertEqual(result["classification"], "compile-failure")
        self.assertIn("error[E0308]", result["output_sample"])

    def test_scoped_files_exist_and_stay_in_repository(self):
        tool_dir = pathlib.Path(__file__).resolve().parent
        root = tool_dir.parents[1]
        manifest = json.loads((tool_dir / "scope.json").read_text("utf-8"))
        self.assertGreater(len(manifest["rustfmt"]), 0)
        for relative in manifest["rustfmt"]:
            path = (root / relative).resolve()
            self.assertTrue(path.is_relative_to(root))
            self.assertTrue(path.is_file(), relative)


if __name__ == "__main__":
    unittest.main()
