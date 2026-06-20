Installation on office:
- Check the current SSD is OK, backup is ok and backup some of the home files
- Fresh fedora ISO on USB with the files
- Gitlab YAML, compositor bin compile within Winit and within native
- Config files:
  - wayland-session
  - services incl. mx-master extension(the whole project dir- incl. setup scripts)
  - zed extension and settings
  - usr/bin/exec+desktop
- Snowies - bundled execs incl. yamls,  AI. and services
- WBCLI, SC, CLUSTER - bundled exec, home directory
- Webstorm plugin
- Installation script - whenever something is missing, add to it and turn back.

DNF History:

Probably required:(not sure)
protobuf
pixma
dnf5daemon-server
Compositor devel: ( add all because some has corresponding non -devel variant )
dnf install pulseaudio-libs-devel
dnf install pam-devel
dnf install -y libinput-devel libseat-devel libxkbcommon-devel 
dnf install protobuf-devel protobuf-compiler
dnf install libseat-devel
dnf group install c-development

General support:
dnf install webkit2gtk4.1-devel openssl-devel curl wget file libappi
dnf install zsh
rustup, cargo
node
jetbrains

Gaming:
dnf install xorg-x11-drv-nvidia-libs.i686

Software:
dnf install blender
dnf install gh

Not sure:
dnf5 --config /kiwi_dnf5.config -y --disable-plugin=priorities,versi 
dnf5 --config /builddir/result/image/build/image-root/kiwi_dnf5.conf 

Nvidia:
dnf install -y nvidia-container-toolkit
dnf install -y nvidia-container-toolkit-1.19.0-1 nvidia-container-to
dnf install xorg-x11-drv-nvidia-cuda                                 
dnf remove golang-github-nvidia-container-toolkit-1.17.4-3.fc43.x86_
dnf install akmod-nvidia                                             
dnf remove akmod-nvidia xorg-x11-drv-nvidia-cuda                     
dnf install akmod-nvidia xorg-x11-drv-nvidia-cuda                    
dnf -y install --nogpgcheck --disablerepo=* /tmp/akmods.FjrjywZj/res  # Probably cause of scripts of automated updates
dnf -y install --nogpgcheck --disablerepo=* /tmp/akmods.VwceAWFi/res # Probably cause of scripts of automated updates
dnf -y install --nogpgcheck --disablerepo=* /tmp/akmods.GJTB5eU7/res # Probably cause of scripts of automated updates
dnf -y install --nogpgcheck --disablerepo=* /tmp/akmods.WN2tt4eY/res # Probably cause of scripts of automated updates
dnf -y install --nogpgcheck --disablerepo=* /tmp/akmods.pHwSjXfF/res # Probably cause of scripts of automated updates
dnf -y install --nogpgcheck --disablerepo=* /tmp/akmods.6Qri6GWK/res # Probably cause of scripts of automated updates
dnf -y install --nogpgcheck --disablerepo=* /tmp/akmods.zHa4EYxs/res  # Probably cause of scripts of automated updates
dnf -y install --nogpgcheck --disablerepo=* /tmp/akmods.rPtWZMmW/res  # Probably cause of scripts of automated updates
dnf -y install --nogpgcheck --disablerepo=* /tmp/akmods.mw2Fx0Ku/res  # Probably cause of scripts of automated updates
dnf -y install --nogpgcheck --disablerepo=* /tmp/akmods.Q9TiVrKO/res  # Probably cause of scripts of automated updates
dnf -y install --nogpgcheck --disablerepo=* /tmp/akmods.BtbdbYtV/res  # Probably cause of scripts of automated updates
dnf -y install --nogpgcheck --disablerepo=* /tmp/akmods.TTDEqnpN/res  # Probably cause of scripts of automated updates
dnf -y install --nogpgcheck --disablerepo=* /tmp/akmods.hqNojfX4/res  # Probably cause of scripts of automated updates
dnf remove *nvidia* # Probably required after default noveu provided by os. could be better actually but need to make sure. The best would be to use nvidia and bujild from source instead of akmod.

Not related
dnf remove gnome-extensions-app                                      
dnf install mutter-devel
