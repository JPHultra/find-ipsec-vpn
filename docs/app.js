// FINDMORE VPN GITHUB PAGES LANDING SCRIPTS

document.addEventListener('DOMContentLoaded', () => {
  // 1. INSTALLER DISTRO TAB SWITCHING LOGIC
  const installerTabBtns = document.querySelectorAll('.installer-tab-btn');
  const codeTextElem = document.getElementById('installer-code-text');

  const installerSnippets = {
    auto: `# Clone repository and run auto-detecting universal installer
git clone https://github.com/JPHultra/findmore-vpn.git
cd findmore-vpn
./install.sh`,

    arch: `# Arch Linux / Omarchy / Manjaro installation
git clone https://github.com/JPHultra/findmore-vpn.git
cd findmore-vpn
./scripts/install/install-arch.sh`,

    debian: `# Ubuntu / Debian / Pop!_OS / Linux Mint installation
git clone https://github.com/JPHultra/findmore-vpn.git
cd findmore-vpn
./scripts/install/install-debian.sh`,

    fedora: `# Fedora / RHEL / Rocky Linux installation
git clone https://github.com/JPHultra/findmore-vpn.git
cd findmore-vpn
./scripts/install/install-fedora.sh`,

    opensuse: `# openSUSE Leap / Tumbleweed installation
git clone https://github.com/JPHultra/findmore-vpn.git
cd findmore-vpn
./scripts/install/install-opensuse.sh`
  };

  installerTabBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      const distro = btn.getAttribute('data-distro');

      installerTabBtns.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');

      if (installerSnippets[distro]) {
        codeTextElem.textContent = installerSnippets[distro];
      }
    });
  });

  // 2. ONE-CLICK CODE COPY LOGIC
  const copyBtn = document.getElementById('btn-copy-code');
  if (copyBtn) {
    copyBtn.addEventListener('click', async () => {
      const textToCopy = codeTextElem.textContent;
      try {
        await navigator.clipboard.writeText(textToCopy);
        const originalText = copyBtn.innerHTML;
        copyBtn.innerHTML = '<span>Copied! ✓</span>';
        copyBtn.style.backgroundColor = '#10b981';
        copyBtn.style.color = '#0b0c10';

        setTimeout(() => {
          copyBtn.innerHTML = originalText;
          copyBtn.style.backgroundColor = '';
          copyBtn.style.color = '';
        }, 2000);
      } catch (err) {
        console.error('Failed to copy text: ', err);
      }
    });
  }
});
