set -x
set -e

# Encumber the account
#export TWITTER_PASSWORD=$(python3 scripts/twitter.py)
. .env
export TWITTER_PASSWORD

# Start the tweeting client
pushd client
cargo run --release &
SERVER=$!
popd

# Login again to pass oauth to the client
python3 scripts/tee.py

wait $SERVER
