#!/data/data/com.termux/files/usr/bin/bash
# CodeIO Termux setup — one command to get from nothing to running.
# Usage:  bash setup.sh
set -e

green() { printf '\033[0;32m%s\033[0m\n' "$1"; }
blue()  { printf '\033[0;34m%s\033[0m\n' "$1"; }

blue "== CodeIO Termux setup =="
green "[1/4] Installing packages (rust, git, protobuf)..."
pkg install -y rust git protobuf

if [ ! -d "$HOME/CodeIO" ]; then
  green "[2/4] Cloning CodeIO..."
  echo -n "Paste your GitHub token (or leave blank if repo is public): "
  read -r TOKEN
  if [ -z "$TOKEN" ]; then
    git clone https://github.com/brandon-roberts/CodeIO.git "$HOME/CodeIO"
  else
    git clone "https://brandon-roberts:${TOKEN}@github.com/brandon-roberts/CodeIO.git" "$HOME/CodeIO"
  fi
else
  green "[2/4] CodeIO already cloned — pulling latest..."
  git -C "$HOME/CodeIO" pull --ff-only || true
fi

green "[3/4] Building codeio (first build is slow — several minutes)..."
cd "$HOME/CodeIO/services"
cargo build -p codeio-cli --release

green "[4/4] Installing the 'cio' launcher..."
BIN="$HOME/CodeIO/services/target/release/codeio"
mkdir -p "$HOME/.local/bin"
cat > "$HOME/.local/bin/cio" << LAUNCH
#!/data/data/com.termux/files/usr/bin/bash
exec "$BIN" "\$@"
LAUNCH
chmod +x "$HOME/.local/bin/cio"
grep -q '.local/bin' "$HOME/.bashrc" 2>/dev/null || echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
export PATH="$HOME/.local/bin:$PATH"

blue "== Done! =="
echo "Try:  cio menu     (interactive menu)"
echo "      cio doctor   (system check)"
echo "      cio run \$HOME/CodeIO/examples/tables.cio"
