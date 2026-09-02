#!/usr/bin/env python3
"""Run the exact candidate through a current Atlas add/readback/delete flow."""

from __future__ import annotations

import argparse
import json
import sys
import tempfile
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--atlas-root", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    atlas_root = args.atlas_root.resolve(strict=True)
    candidate = args.candidate.resolve(strict=True)
    sys.path.insert(0, str(atlas_root))

    from fastapi.testclient import TestClient

    from backend.app import create_app
    from backend.config import AtlasPaths
    from backend.interfaces.base import InterfaceRegistry

    with tempfile.TemporaryDirectory(prefix="atlas-v094-compat-") as directory:
        root = Path(directory)
        paths = AtlasPaths(
            data_dir=root / "data",
            database=root / "data" / "atlas.db",
            workspace=root / "workspace",
        )
        app = create_app(paths=paths, registry=InterfaceRegistry([]))
        with TestClient(app) as client:
            response = client.post(
                "/api/mcp/servers",
                json={
                    "command": str(candidate),
                    "arguments": [],
                    "workingDirectory": str(candidate.parent),
                },
            )
            record = response.json()
            assert response.status_code == 201, record
            assert record["protocolVersion"] == "2025-03-26", record
            assert record["status"] == "ready", record
            assert len(record["tools"]) == 10, record
            assert all(tool["readOnly"] is True for tool in record["tools"]), record
            assert client.get("/api/mcp/servers").json() == [record]

            server_id = record["id"]
            deleted = client.delete(f"/api/mcp/servers/{server_id}")
            assert deleted.status_code == 204
            assert client.get("/api/mcp/servers").json() == []

            print(
                json.dumps(
                    {
                        "addStatus": response.status_code,
                        "allReadOnly": True,
                        "deletedStatus": deleted.status_code,
                        "persistedReadback": True,
                        "protocolVersion": record["protocolVersion"],
                        "remainingRegistrations": 0,
                        "serverStatus": record["status"],
                        "toolCount": len(record["tools"]),
                    },
                    sort_keys=True,
                )
            )


if __name__ == "__main__":
    main()
