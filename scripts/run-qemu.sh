#!/bin/bash
# SPDX-License-Identifier: BSD-3-Clause
set +e

_SOURCE="${BASH_SOURCE[0]}"
while [ -L "$_SOURCE" ]; do # resolve $_SOURCE until the file is no longer a symlink
  DIR=$( cd -P "$( dirname "${_SOURCE}" )" >/dev/null 2>&1 && pwd )
  _SOURCE=$(readlink "$_SOURCE")
  [[ $_SOURCE != /* ]] && _SOURCE="${DIR}/${_SOURCE}" # if $_SOURCE was a relative symlink, we need to resolve it relative to the path where the symlink file was located
done
SRC_DIR=$( cd -P "$( dirname "${_SOURCE}" )/../" >/dev/null 2>&1 && pwd )
TARGET_DIR="${SRC_DIR}/target"

EFI_ROOT="${TARGET_DIR}/esp"
EFI_BOOT_DIR="${EFI_ROOT}/EFI/boot"
OVMF_DIR="${TARGET_DIR}/.ovmf"

[ ! -d "${EFI_BOOT_DIR}" ] && mkdir -p "${EFI_BOOT_DIR}"

OVMF_CODE_FILE="${OVMF_DIR}/OVMF_CODE.4m.fd"
OVMF_VARS_FILE="${OVMF_DIR}/OVMF_VARS.4m.fd"

[ ! -f "${OVMF_CODE_FILE}" ] && bash "${SRC_DIR}/scripts/build-ovmf.sh"

OVMF_GDB_MAP="${OVMF_DIR}/gdb-script"
[ ! -f "${OVMF_GDB_MAP}" ] && bash "${SRC_DIR}/scripts/make-dbg.sh"


TARGET="debug"
QEMU_ARGS=""

while getopts "rd" OPTS; do
	case "$OPTS" in
		r)
			TARGET="release"
			;;
		d)
			QEMU_ARGS="${QEMU_ARGS} -S -s"
			;;
		\?)
			echo "Unknown Option"
			exit
			;;
	esac
	shift
done

echo " * Target Dir: ${TARGET_DIR}"
echo " * Target Configuration: ${TARGET}"
echo " * Extra QEMU Args: ${QEMU_ARGS}"

TAPERIPPER_IMG="${TARGET_DIR}/x86_64-unknown-uefi/${TARGET}/taperipper.efi"
BOOT_IMG="${EFI_BOOT_DIR}/BOOTx64.efi"

UNWIND_FILE="${TAPERIPPER_IMG}.unwind"
SECTIONS_FILE="${TAPERIPPER_IMG}.sections"

if [ -f "${TAPERIPPER_IMG}" ]; then
	llvm-readobj --unwind "${TAPERIPPER_IMG}" > "${UNWIND_FILE}"
	llvm-readobj -S "${TAPERIPPER_IMG}" > "${SECTIONS_FILE}"

	# IMG_BASE_ADDR="$(llvm-readobj --file-header ${TAPERIPPER_IMG} | grep ImageBase | cut -d ':' -f 2 | tr -d [:space:])"
	IMG_BASE_ADDR="0x00005E7D000"

	TXT_LOAD_ADDR="$(cat ${SECTIONS_FILE} | grep -A 3 \\.text | grep VirtualAddress | cut -d ':' -f 2 | tr -d [:space:])"
	TXT_ADDR_CALC="obase=16;ibase=16;${IMG_BASE_ADDR#"0x"}+${TXT_LOAD_ADDR#"0x"}"
	TXT_ADDR="0x$(echo ${TXT_ADDR_CALC} | bc)"

	DATA_LOAD_ADDR="$(cat ${SECTIONS_FILE} | grep -A 3 \\.data | grep VirtualAddress | cut -d ':' -f 2 | tr -d [:space:])"
	DATA_ADDR_CALC="obase=16;ibase=16;${IMG_BASE_ADDR#"0x"}+${DATA_LOAD_ADDR#"0x"}"
	DATA_ADDR="0x$(echo ${DATA_ADDR_CALC} | bc)"

	RDATA_LOAD_ADDR="$(cat ${SECTIONS_FILE} | grep -A 3 \\.rdata | grep VirtualAddress | cut -d ':' -f 2 | tr -d [:space:])"
	RDATA_ADDR_CALC="obase=16;ibase=16;${IMG_BASE_ADDR#"0x"}+${RDATA_LOAD_ADDR#"0x"}"
	RDATA_ADDR="0x$(echo ${RDATA_ADDR_CALC} | bc)"
fi

echo "Image Base: ${IMG_BASE_ADDR}"
echo " * .text: ${TXT_ADDR}"
echo " * .data: ${DATA_ADDR}"
echo " * .rdata: ${RDATA_ADDR}"

GDBSCRIPT="${TARGET_DIR}/gdbscript"

cat "${OVMF_GDB_MAP}" > "${GDBSCRIPT}"
echo "add-symbol-file ${TAPERIPPER_IMG} -s .text ${TXT_ADDR} -s .data ${DATA_ADDR} -s .rdata ${RDATA_ADDR}" >> "${GDBSCRIPT}"
echo "tar remote 127.0.0.1:1234" >> "${GDBSCRIPT}"
echo "display /5i \$pc" >> "${GDBSCRIPT}"

rm "${BOOT_IMG}"
cp "${TAPERIPPER_IMG}" "${BOOT_IMG}"

pushd "${TARGET_DIR}" || exit

# -global isa-debugcon.iobase=0x402 \#
qemu-system-x86_64 -enable-kvm \
	-debugcon stdio $QEMU_ARGS \
	-rtc base=localtime,clock=rt \
	-drive if=pflash,format=raw,readonly=on,file="${OVMF_CODE_FILE}" \
	-drive if=pflash,format=raw,readonly=on,file="${OVMF_VARS_FILE}" \
	-drive format=raw,file=fat:rw:"${EFI_ROOT}" | tee log.txt

popd || exit
