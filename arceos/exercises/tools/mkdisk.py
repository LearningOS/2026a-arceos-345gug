#!/usr/bin/env python3
"""Create a small FAT32 image and copy files into it.

The ArceOS FAT filesystem only needs a very small subset of FAT32.  This
script builds the image from scratch so that the exercises do not depend on
the external `mtools` package (mcopy/mmd), which is often not installed.

Usage:
    mkdisk.py OUT.img /sbin/mapfile=path/to/mapfile.elf [/sbin/other=...]

Only 8.3 file names are supported, which is enough for `sbin`, `mapfile` and
`skernel2`.  The image is 64 MiB with 512-byte clusters.
"""

import os
import struct
import sys

SECTOR = 512
SPC = 1                      # sectors per cluster (512 B)
# A one-sector cluster keeps the cluster count above 65525, which is what
# rust-fatfs requires to classify a volume as FAT32.
RESERVED = 32                # reserved sectors
NUM_FATS = 2
IMAGE_MB = 64
VOLUME_ID = 0x20261123
DATE = ((2026 - 1980) << 9) | (11 << 5) | 23
TIME = 0

FAT_EOC = 0x0FFFFFFF
FAT_MEDIA = 0x0FFFFFF8


def die(msg):
    print("mkdisk.py: " + msg, file=sys.stderr)
    sys.exit(1)


def ceil_div(a, b):
    return (a + b - 1) // b


def split_83(name):
    name = name.upper()
    base, dot, ext = name.rpartition(".")
    if not dot:
        base, ext = name, ""
    if not base or len(base) > 8 or len(ext) > 3:
        die("name %r cannot be represented as 8.3" % name)
    if any(c in base + ext for c in ' "*/:<>?\\|+,'):
        die("invalid FAT name %r" % name)
    return (base.ljust(8) + ext.ljust(3)).encode("ascii")


def dirent(name, attr, cluster, size):
    return struct.pack(
        "<11sBBBHHHHHHHI",
        name,
        attr,
        0,          # NT reserved
        0,          # creation time, tenths
        TIME,       # creation time
        DATE,       # creation date
        DATE,       # last access date
        (cluster >> 16) & 0xFFFF,
        TIME,       # last write time
        DATE,       # last write date
        cluster & 0xFFFF,
        size,
    )


class Dir:
    def __init__(self, name, parent):
        self.name = name
        self.parent = parent
        self.children = {}
        self.clusters = []


class File:
    def __init__(self, name, host, size):
        self.name = name
        self.host = host
        self.size = size
        self.clusters = []


def add_file(root, image_path, host_path):
    parts = [p for p in image_path.strip("/").split("/") if p]
    if not parts:
        die("empty image path")
    cur = root
    for part in parts[:-1]:
        nxt = cur.children.get(part)
        if nxt is None:
            nxt = Dir(part, cur)
            cur.children[part] = nxt
        elif not isinstance(nxt, Dir):
            die("%s is both a file and a directory" % part)
        cur = nxt
    leaf = parts[-1]
    if leaf in cur.children:
        die("duplicate image path %s" % image_path)
    if not os.path.isfile(host_path):
        die("missing host file %s" % host_path)
    cur.children[leaf] = File(leaf, host_path, os.path.getsize(host_path))


class Allocator:
    def __init__(self):
        self.next_cluster = 2

    def chain(self, count):
        if count <= 0:
            return []
        start = self.next_cluster
        self.next_cluster += count
        return list(range(start, start + count))


def layout_dirs(d, alloc):
    entries = len(d.children) + (0 if d.parent is None else 2)
    count = max(1, ceil_div(entries * 32, SECTOR * SPC))
    d.clusters = alloc.chain(count)
    for child in d.children.values():
        if isinstance(child, Dir):
            layout_dirs(child, alloc)


def layout_files(d, alloc):
    for child in d.children.values():
        if isinstance(child, File):
            child.clusters = alloc.chain(ceil_div(child.size, SECTOR * SPC))
        else:
            layout_files(child, alloc)


def cluster_offset(cluster, first_data_sector):
    return (first_data_sector + (cluster - 2) * SPC) * SECTOR


def write_dir(data, d, first_data_sector):
    entries = []
    if d.parent is not None:
        entries.append(dirent(b"." + b" " * 10, 0x10, d.clusters[0], 0))
        parent_cluster = 0 if d.parent.parent is None else d.parent.clusters[0]
        entries.append(dirent(b".." + b" " * 9, 0x10, parent_cluster, 0))
    for name, child in sorted(d.children.items()):
        if isinstance(child, Dir):
            entries.append(dirent(split_83(name), 0x10, child.clusters[0], 0))
        else:
            entries.append(dirent(split_83(name), 0x20, child.clusters[0], child.size))

    # Directories are stored as a chain of clusters; spread entries over them.
    per_cluster = (SECTOR * SPC) // 32
    for idx, cluster in enumerate(d.clusters):
        off = cluster_offset(cluster, first_data_sector)
        chunk = entries[idx * per_cluster:(idx + 1) * per_cluster]
        for i, ent in enumerate(chunk):
            data[off + i * 32:off + i * 32 + 32] = ent
    if len(entries) > len(d.clusters) * per_cluster:
        die("directory %s does not fit" % d.name)
    for child in d.children.values():
        if isinstance(child, Dir):
            write_dir(data, child, first_data_sector)


def write_files(data, d, first_data_sector):
    for child in d.children.values():
        if isinstance(child, File):
            with open(child.host, "rb") as f:
                content = f.read()
            if len(content) != child.size:
                die("file %s changed while building the image" % child.host)
            # The simple allocator returns consecutive clusters, so the data
            # is contiguous in the image.
            if child.clusters:
                off = cluster_offset(child.clusters[0], first_data_sector)
                data[off:off + len(content)] = content
        else:
            write_files(data, child, first_data_sector)


def build_image(out_path, specs):
    total_sectors = IMAGE_MB * 1024 * 1024 // SECTOR
    root = Dir("", None)
    for spec in specs:
        if "=" not in spec:
            die("expected IMAGE_PATH=HOST_FILE, got %r" % spec)
        image_path, host_path = spec.split("=", 1)
        add_file(root, image_path, host_path)

    # Work out the FAT size (iteratively, like mkfs.fat does).
    fat_sectors = 1
    while True:
        data_sectors = total_sectors - RESERVED - NUM_FATS * fat_sectors
        clusters = data_sectors // SPC
        need = ceil_div((clusters + 2) * 4, SECTOR)
        if need <= fat_sectors:
            break
        fat_sectors = need

    total_clusters = (total_sectors - RESERVED - NUM_FATS * fat_sectors) // SPC
    if total_clusters >= 0x0FFFFFF5:
        die("too many clusters for FAT32")

    alloc = Allocator()
    layout_dirs(root, alloc)
    if root.clusters[0] != 2:
        die("internal error: root cluster is not 2")
    layout_files(root, alloc)
    used_clusters = alloc.next_cluster - 2
    if used_clusters > total_clusters:
        die("image is too small")

    first_data_sector = RESERVED + NUM_FATS * fat_sectors
    data = bytearray(total_sectors * SECTOR)

    # --- boot sector -----------------------------------------------------
    bs = bytearray(SECTOR)
    bs[0:3] = b"\xeb\x58\x90"
    bs[3:11] = b"MSWIN4.1"
    struct.pack_into("<HBHBHHBHHHII", bs, 0x0B,
                     SECTOR, SPC, RESERVED, NUM_FATS, 0, 0, 0xF8,
                     0, 63, 255, 0, total_sectors)
    struct.pack_into("<IHHIHH", bs, 0x24,
                     fat_sectors, 0, 0, 2, 1, 6)
    bs[0x40] = 0x80           # drive number
    bs[0x42] = 0x29           # extended boot signature
    struct.pack_into("<I", bs, 0x43, VOLUME_ID)
    bs[0x47:0x52] = b"ARCEOS     "
    bs[0x52:0x5A] = b"FAT32   "
    struct.pack_into("<H", bs, 0x1FE, 0xAA55)
    data[0:SECTOR] = bs
    data[6 * SECTOR:7 * SECTOR] = bs

    # --- FSInfo ----------------------------------------------------------
    fsinfo = bytearray(SECTOR)
    struct.pack_into("<I", fsinfo, 0, 0x41615252)
    struct.pack_into("<I", fsinfo, 484, 0x61417272)
    struct.pack_into("<I", fsinfo, 488, total_clusters - used_clusters)
    struct.pack_into("<I", fsinfo, 492, alloc.next_cluster)
    struct.pack_into("<I", fsinfo, 508, 0xAA550000)
    data[1 * SECTOR:2 * SECTOR] = fsinfo
    data[7 * SECTOR:8 * SECTOR] = fsinfo

    # --- FAT -------------------------------------------------------------
    fat = [0] * (total_clusters + 2)
    fat[0] = FAT_MEDIA
    fat[1] = FAT_EOC
    chains = []

    def collect(d):
        chains.append(d.clusters)
        for child in d.children.values():
            if isinstance(child, Dir):
                collect(child)
            else:
                chains.append(child.clusters)

    collect(root)
    for chain in chains:
        for a, b in zip(chain, chain[1:]):
            fat[a] = b
        if chain:
            fat[chain[-1]] = FAT_EOC

    fat_bytes = b"".join(struct.pack("<I", v & 0x0FFFFFFF) for v in fat)
    fat_bytes += b"\x00" * (fat_sectors * SECTOR - len(fat_bytes))
    for i in range(NUM_FATS):
        off = (RESERVED + i * fat_sectors) * SECTOR
        data[off:off + len(fat_bytes)] = fat_bytes

    # --- directory entries and file data --------------------------------
    write_dir(data, root, first_data_sector)
    write_files(data, root, first_data_sector)

    with open(out_path, "wb") as f:
        f.write(data)
    print("mkdisk.py: wrote %s (%d MiB, %d clusters used)"
          % (out_path, IMAGE_MB, used_clusters))


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    build_image(sys.argv[1], sys.argv[2:])


if __name__ == "__main__":
    main()
