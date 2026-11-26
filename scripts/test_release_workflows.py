"""Keep the workflow trigger policy machine-readable without a YAML dependency."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]


def triggers(filename):
    # JSON flow mappings are valid YAML; only the event declarations use this form.
    lines = (ROOT / ".github/workflows" / filename).read_text().splitlines()
    declarations = [line[3:].strip() for line in lines if line.startswith("on:")]
    if len(declarations) != 1:
        raise ValueError("Expected one JSON workflow event declaration")
    return json.loads(declarations[0])


class WorkflowTests(unittest.TestCase):
    def test_only_release_tags_can_start_a_workflow(self):
        self.assertEqual(triggers("release.yml"), {"push": {"tags": ["v*"]}})
        self.assertEqual(triggers("windows-experiment.yml"), {
            "workflow_call": {"inputs": {"ref": {"type": "string", "required": True}}}})
        self.assertEqual({p.name for p in (ROOT / ".github/workflows").glob("*.yml")},
                         {"release.yml", "windows-experiment.yml"})
