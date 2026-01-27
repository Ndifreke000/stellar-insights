#!/bin/bash

# This script sets up cron jobs for automated backups
# Run this as root or with sudo

SCRIPT_DIR=$(dirname "$(realpath "$0")")

# Daily full backup at 2 AM
(crontab -l ; echo "0 2 * * * cd $SCRIPT_DIR && ./full_backup.sh") | crontab -

# Hourly incremental backup
(crontab -l ; echo "0 * * * * cd $SCRIPT_DIR && ./incremental_backup.sh") | crontab -

echo "Cron jobs set up successfully."