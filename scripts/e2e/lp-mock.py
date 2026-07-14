#!/usr/bin/env python3
"""Tiny local mock of Launchpad's getPublishedSources API.

Serves canned responses for scripts/e2e/lp-skip-test.sh; the scenario
is picked by the query's source_name:

  accepted    one Published entry for the noble series
  pending     one Pending entry for the noble series
  superseded  one Superseded entry (not an acceptance)
  otherseries one Published entry, but for jammy
  malformed   a non-JSON body
  flaky       empty on the first request, Published from the second on
              (the "client failed but Launchpad accepted" recheck)
  broken      HTTP 500
  anything else: an empty entries list

Binds 127.0.0.1 on an ephemeral port and prints the port on stdout,
then serves until killed. No network access beyond loopback.
"""

import json
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import parse_qs, urlparse

SERIES_BASE = "https://api.launchpad.net/1.0/ubuntu/"
flaky_hits = {"n": 0}


def entries(status, series):
    return {
        "entries": [
            {"status": status, "distro_series_link": SERIES_BASE + series}
        ]
    }


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        q = parse_qs(urlparse(self.path).query)
        name = (q.get("source_name") or [""])[0]
        status = 200
        if name == "accepted":
            body = json.dumps(entries("Published", "noble")).encode()
        elif name == "pending":
            body = json.dumps(entries("Pending", "noble")).encode()
        elif name == "superseded":
            body = json.dumps(entries("Superseded", "noble")).encode()
        elif name == "otherseries":
            body = json.dumps(entries("Published", "jammy")).encode()
        elif name == "malformed":
            body = b"{this is not json"
        elif name == "flaky":
            flaky_hits["n"] += 1
            if flaky_hits["n"] == 1:
                body = json.dumps({"entries": []}).encode()
            else:
                body = json.dumps(entries("Published", "noble")).encode()
        elif name == "broken":
            status = 500
            body = b"internal error"
        else:
            body = json.dumps({"entries": []}).encode()
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):  # quiet: the test log is the output
        pass


def main():
    server = HTTPServer(("127.0.0.1", 0), Handler)
    print(server.server_address[1], flush=True)
    server.serve_forever()


if __name__ == "__main__":
    sys.exit(main())
