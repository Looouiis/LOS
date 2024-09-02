# user/build.py

import os

base_address = 0x80400000
step = 0x20000
linker = 'user/src/linker.ld'

app_id = 0
apps = os.listdir('user/src/bin')
apps.sort()
target = 'target/riscv64gc-unknown-none-elf/release'
for app in apps:
    app = app[:app.find('.')]
    lines = []
    lines_before = []
    with open(linker, 'r') as f:
        for line in f.readlines():
            lines_before.append(line)
            line = line.replace(hex(base_address), hex(base_address+step*app_id))
            lines.append(line)
    with open(linker, 'w+') as f:
        f.writelines(lines)
    os.system('cargo build --target riscv64gc-unknown-none-elf --package user --bin %s --release' % app)
    tmp = target + '/' + app
    os.system('rust-objcopy --strip-all -O binary %s %s.bin' % (tmp, tmp))
    print('[build.py] application %s start with address %s' %(app, hex(base_address+step*app_id)))
    with open(linker, 'w+') as f:
        f.writelines(lines_before)
    app_id = app_id + 1