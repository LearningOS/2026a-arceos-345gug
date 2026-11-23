TARGET := mapfile
ELF := $(TARGET).elf

RUSTC ?= rustc
TARGET_TRIPLE := riscv64gc-unknown-none-elf
RUSTFLAGS := --edition=2021 \
             --target $(TARGET_TRIPLE) \
             -C panic=abort \
             -C opt-level=z \
             -C debuginfo=0 \
             -C relocation-model=static \
             -C link-arg=-Tlinker.lds \
             -C link-arg=-no-pie \
             -C link-arg=--no-relax

all: $(ELF)

$(ELF): src/main.rs linker.lds
	$(RUSTC) $(RUSTFLAGS) -o $@ src/main.rs

clean:
	rm -f $(ELF)

.PHONY: all clean
