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

[ ! -d "${EFI_BOOT_DIR}" ] && mkdir -p "${EFI_BOOT_DIR}"

OVMF_CODE_FILE="${TARGET_DIR}/OVMF_CODE.4m.fd"
OVMF_VARS_FILE="${TARGET_DIR}/OVMF_VARS.4m.fd"

[ ! -f "${OVMF_CODE_FILE}" ] && cp "/usr/share/edk2/x64/OVMF_CODE.4m.fd" "${OVMF_CODE_FILE}"
[ ! -f "${OVMF_VARS_FILE}" ] && cp "/usr/share/edk2/x64/OVMF_VARS.4m.fd" "${OVMF_VARS_FILE}"


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

if [ -f "${TAPERIPPER_IMG}" ]; then
	IMG_BASE_ADDR="$(llvm-readobj --file-header ${TAPERIPPER_IMG} | grep ImageBase | cut -d ':' -f 2 | tr -d [:space:])"

	TXT_LOAD_ADDR="$(llvm-readobj -S ${TAPERIPPER_IMG} | grep -A 3 \\.text | grep VirtualAddress | cut -d ':' -f 2 | tr -d [:space:])"
	TXT_ADDR_CALC="obase=16;ibase=16;${IMG_BASE_ADDR#"0x"}+${TXT_LOAD_ADDR#"0x"}"
	TXT_ADDR="0x$(echo ${TXT_ADDR_CALC} | bc)"

	DATA_LOAD_ADDR="$(llvm-readobj -S ${TAPERIPPER_IMG} | grep -A 3 \\.data | grep VirtualAddress | cut -d ':' -f 2 | tr -d [:space:])"
	DATA_ADDR_CALC="obase=16;ibase=16;${IMG_BASE_ADDR#"0x"}+${DATA_LOAD_ADDR#"0x"}"
	DATA_ADDR="0x$(echo ${DATA_ADDR_CALC} | bc)"

	RDATA_LOAD_ADDR="$(llvm-readobj -S ${TAPERIPPER_IMG} | grep -A 3 \\.rdata | grep VirtualAddress | cut -d ':' -f 2 | tr -d [:space:])"
	RDATA_ADDR_CALC="obase=16;ibase=16;${IMG_BASE_ADDR#"0x"}+${RDATA_LOAD_ADDR#"0x"}"
	RDATA_ADDR="0x$(echo ${RDATA_ADDR_CALC} | bc)"
fi

echo "Image Base: ${IMG_BASE_ADDR}"
echo " * .text: ${TXT_ADDR}"
echo " * .data: ${DATA_ADDR}"
echo " * .rdata: ${RDATA_ADDR}"

rm "${BOOT_IMG}"
cp "${TAPERIPPER_IMG}" "${BOOT_IMG}"

pushd "${TARGET_DIR}" || exit

qemu-system-x86_64 -enable-kvm \
	-debugcon stdio $QEMU_ARGS \
	-rtc base=localtime,clock=rt \
	-drive if=pflash,format=raw,readonly=on,file="${OVMF_CODE_FILE}" \
	-drive if=pflash,format=raw,readonly=on,file="${OVMF_VARS_FILE}" \
	-drive format=raw,file=fat:rw:"${EFI_ROOT}"

popd || exit
