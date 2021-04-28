#!/bin/sh

echo "Creating ~/raft directory..."
mkdir -p ~/raft

echo "Copying hosts.txt..."
cp config/hosts.txt ~/raft
cp config/hosts_client.txt ~/raft
