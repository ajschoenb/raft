#!/bin/sh

echo "Creating ~/raft directory..."
mkdir -p ~/raft

echo "Copying hosts.txt..."
cp ./hosts.txt ~/raft
cp ./hosts_client.txt ~/raft
