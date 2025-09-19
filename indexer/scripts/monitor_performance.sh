#!/bin/bash

echo "🚀 Charms Indexer Performance Monitor"
echo "======================================"

while true; do
    # Get current time
    timestamp=$(date '+%H:%M:%S')
    
    # Get indexer process info
    indexer_pid=$(pgrep -f "charms-indexer" | head -1)
    
    if [ -n "$indexer_pid" ]; then
        # Get CPU and memory usage
        cpu_mem=$(ps -p $indexer_pid -o %cpu,%mem --no-headers)
        
        # Get system load
        load=$(uptime | awk -F'load averages: ' '{print $2}' | awk '{print $1}')
        
        # Get network connections
        connections=$(lsof -p $indexer_pid 2>/dev/null | grep TCP | wc -l | tr -d ' ')
        
        # Get recent log count (transactions processed in last second)
        tx_count=$(tail -100 /dev/null 2>/dev/null || echo "0")
        
        echo "[$timestamp] CPU: $cpu_mem% | Load: $load | Connections: $connections | PID: $indexer_pid"
    else
        echo "[$timestamp] ❌ Indexer not running"
    fi
    
    sleep 2
done
