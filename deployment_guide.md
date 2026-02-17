# VPS Deployment Guide: CZ Sniper Bot

To run your bot 24/7 on a Linux VPS, we recommend one of the three following methods.

## Prerequisites (Linux Dependencies)
Before building the bot, you must install the OpenSSL development headers. Run the command that matches your VPS operating system:

**For Ubuntu / Debian:**
```bash
sudo apt update
sudo apt install build-essential libssl-dev pkg-config
```

**For CentOS / RHEL / Fedora:**
```bash
sudo yum install openssl-devel
```

| Feature | Systemd (Method 1) | Tmux (Method 2) | Nohup (Method 3) |
| :--- | :---: | :---: | :---: |
| Runs in Background | ✅ | ✅ | ✅ |
| **Restarts on Crash** | ✅ | ❌ | ❌ |
| **Restarts on VPS Reboot** | ✅ | ❌ | ❌ |
| Interactive Terminal | ❌ | ✅ | ❌ |

---

## Method 1: The "Pro" Way (Systemd)
**Best for Production.** This is the **only** method that automatically restarts the bot if it crashes or if the VPS reboots.

1. **Build the production binary** on your VPS:
   ```bash
   cargo build --release --bin bsc-discord-sniper
   ```

2. **Create a service file**:
   ```bash
   sudo nano /etc/systemd/system/cz-sniper.service
   ```

3. **Paste this configuration** (update paths to match your VPS):
   ```ini
   [Unit]
   Description=CZ Discord Sniper Bot
   After=network.target

   [Service]
   Type=simple
   User=root
   WorkingDirectory=/root/czsniper
   ExecStart=/root/czsniper/target/release/bsc-discord-sniper
   Restart=always
   RestartSec=5
   Environment=RUST_LOG=info

   [Install]
   WantedBy=multi-user.target
   ```

   > [!IMPORTANT]
   > On Linux, the home directory for **root** is usually `/root/`, not `/home/root/`. 
   > To be 100% sure of your path, go into the `czsniper` folder and run `pwd`. Use that exact output for `WorkingDirectory`.

4. **Start and Enable**:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable cz-sniper
   sudo systemctl start cz-sniper
   ```

5. **Check Logs**:
   ```bash
   journalctl -u cz-sniper -f
   ```

---

## Method 2: The "Quick" Way (Tmux)
**Best for Debugging.** Tmux stays running when you close SSH, but it **will not restart** the bot if the process crashes or if the server reboots.

1. **Start a new session**:
   ```bash
   tmux new -s sniper
   ```

2. **Run the bot**:
   ```bash
   cargo run --release --bin bsc-discord-sniper
   ```

3. **Detach**: Press `Ctrl + B`, then `D`.
   The bot is now running in the background.

4. **Reattach later**:
   ```bash
   tmux attach -t sniper
   ```

---

## Method 3: The "Simple" Way (Nohup)
**Best for one-off runs.** `nohup` (No Hang Up) lets you run a command that keeps going after you logout, but like Tmux, it has **no auto-restart** capability.

1. **Run in background**:
   ```bash
   nohup cargo run --release --bin bsc-discord-sniper > bot.log 2>&1 &
   ```

2. **Stop the bot**:
   ```bash
   pgrep bsc-discord-sniper | xargs kill
   ```

---

## Recommendation
If you are serious about sniping, **Method 1 (Systemd)** is the standard. If your VPS reboots for maintenance, the bot will be back up and monitoring in seconds without you doing anything.

---

## Helpful VPS Tips

### 1. How to find your Username/Path
If you are unsure what to put in the `User` or `WorkingDirectory` fields for Method 1, run these on your VPS:

- **To find your username**: 
  ```bash
  whoami
  ```
- **To find your current directory (Full Path)**: 
  ```bash
  pwd
  ```

### 2. Is my username "root"?
It depends on your provider. 
- If you log in and see `root@vps:~#`, your username is **root**.
- If you see `ubuntu@ip-123-x-x-x:~$`, your username is **ubuntu**.

> [!TIP]
> While `root` works, it has "god mode" permissions. If you have a choice, using a standard user like `ubuntu` or `debian` is slightly more secure for running bots.
