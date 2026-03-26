#!/bin/bash
# VARCAVIA API — 10 curl examples
# Start a node first: cargo run --bin varcavia-node -- --port 8080

BASE="http://localhost:8080"

echo "=== 1. Health check ==="
curl -s "$BASE/health" | python3 -m json.tool

echo -e "\n=== 2. Verify a fact ==="
curl -s "$BASE/api/v1/verify?fact=Earth+diameter+is+12742+km" | python3 -m json.tool

echo -e "\n=== 3. Insert climate data ==="
curl -s -X POST "$BASE/api/v1/data" \
  -H 'Content-Type: application/json' \
  -d '{"content":"Roma: temperature 22C, humidity 65%","domain":"climate","source":"sensor-01"}' \
  | python3 -m json.tool

echo -e "\n=== 4. Insert health data ==="
curl -s -X POST "$BASE/api/v1/data" \
  -H 'Content-Type: application/json' \
  -d '{"content":"Average heart rate during sleep: 60 bpm","domain":"health","source":"study-2024"}' \
  | python3 -m json.tool

echo -e "\n=== 5. Query climate domain ==="
curl -s -X POST "$BASE/api/v1/data/query" \
  -H 'Content-Type: application/json' \
  -d '{"query":"","domain":"climate","limit":10}' \
  | python3 -m json.tool

echo -e "\n=== 6. Get node status ==="
curl -s "$BASE/api/v1/node/status" | python3 -m json.tool

echo -e "\n=== 7. Get public stats ==="
curl -s "$BASE/api/v1/stats" | python3 -m json.tool

echo -e "\n=== 8. Verify data integrity ==="
curl -s -X POST "$BASE/api/v1/data/verify" \
  -H 'Content-Type: application/json' \
  -d '{"id":"REPLACE_WITH_ACTUAL_ID","content":"Roma: temperature 22C, humidity 65%"}' \
  | python3 -m json.tool

echo -e "\n=== 9. Translate JSON to XML ==="
curl -s -X POST "$BASE/api/v1/translate" \
  -H 'Content-Type: application/json' \
  -d '{"data":{"city":"Roma","temp":22},"from_format":"json","to_format":"xml"}' \
  | python3 -m json.tool

echo -e "\n=== 10. Network health ==="
curl -s "$BASE/api/v1/network/health" | python3 -m json.tool
