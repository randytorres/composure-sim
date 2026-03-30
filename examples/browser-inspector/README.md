# Browser Inspector

This is a zero-build static inspector for saved composure artifacts.

Serve the repo root and open the inspector in a browser:

```bash
python3 -m http.server 8000
```

Then visit:

```text
http://127.0.0.1:8000/examples/browser-inspector/
```

Load one or more JSON artifacts with the file picker. Supported shapes:

- `RunSummary`
- `TrajectoryComparison`
- `DeterministicReport`
- `CalibrationResult`
