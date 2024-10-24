set -x
set -e

pushd client
cargo run --release &
SERVER=$!
popd

python3 scripts/tee.py

wait $SERVER
