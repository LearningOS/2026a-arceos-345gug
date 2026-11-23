TARGET := mapfile

CC := riscv64-linux-musl-gcc
STRIP := riscv64-linux-musl-strip

all: $(TARGET)

# Prefer the reference C payload, but fall back to the freestanding Rust
# payload when the musl cross-compiler is not installed.  Both programs perform
# the same open/write/mmap/read/exit sequence, so the kernel test is identical.
$(TARGET): mapfile.c
	@if command -v $(CC) >/dev/null 2>&1; then \
		echo "$(CC) -static mapfile.c -o $@"; \
		$(CC) -static mapfile.c -o $@ && $(STRIP) $@; \
	else \
		echo "warning: $(CC) not found, building the Rust mapfile payload"; \
		$(MAKE) -C ../mapfile && cp ../mapfile/mapfile.elf $@; \
	fi

clean:
	@rm -f $(TARGET)
