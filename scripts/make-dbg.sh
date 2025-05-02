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

OVMF_DIR="${TARGET_DIR}/.ovmf"
OVMF_IMG_DIR="${TARGET_DIR}/.ovmf/efi"

OVMF_CODE_FILE="${OVMF_DIR}/OVMF_CODE.4m.fd"
OVMF_VARS_FILE="${OVMF_DIR}/OVMF_VARS.4m.fd"
OVMF_DEBUG_LOG="${OVMF_DIR}/debug.log"
OVMF_GDB_MAP="${OVMF_DIR}/gdb-script"

if [ ! -f "${OVMF_DEBUG_LOG}" ]; then
	timeout 10 qemu-system-x86_64 -enable-kvm \
		-debugcon "file:${OVMF_DEBUG_LOG}" -global isa-debugcon.iobase=0x402 \
		-drive if=pflash,format=raw,readonly=on,file="${OVMF_CODE_FILE}" \
		-drive if=pflash,format=raw,readonly=on,file="${OVMF_VARS_FILE}"
fi

if [ ! -f "${OVMF_GDB_MAP}" ]; then
	cat ${OVMF_DEBUG_LOG} | grep "Loading" | grep -i "efi" | while read LINE; do
		echo "${LINE}"
		BASE="`echo ${LINE} | cut -d " " -f4`"
		NAME="`echo ${LINE} | cut -d " " -f6 | tr -d "[:cntrl:]"`"
		EFI_FILE="${OVMF_IMG_DIR}/${NAME}"
		DBG_FILE="${EFI_FILE/.efi/.debug}"
		if [ -f "${EFI_FILE}" ]; then
			TXT_LOAD_ADDR="$(llvm-readobj -S ${EFI_FILE} | grep -A 3 \\.text | grep VirtualAddress | cut -d ':' -f 2 | tr -d [:space:])"
			TXT_ADDR_CALC="obase=16;ibase=16;${BASE#"0x"}+${TXT_LOAD_ADDR#"0x"}"
			TXT_ADDR="0x$(echo ${TXT_ADDR_CALC} | bc)"

			DATA_LOAD_ADDR="$(llvm-readobj -S ${EFI_FILE} | grep -A 3 \\.data | grep VirtualAddress | cut -d ':' -f 2 | tr -d [:space:])"
			DATA_ADDR_CALC="obase=16;ibase=16;${BASE#"0x"}+${DATA_LOAD_ADDR#"0x"}"
			DATA_ADDR="0x$(echo ${DATA_ADDR_CALC} | bc)"

			echo "add-symbol-file ${DBG_FILE} ${TXT_ADDR} -s .data ${DATA_ADDR}"  >> "${OVMF_GDB_MAP}"
		else
			>&2 echo "Did not find object ${EFI_FILE}"
		fi
	done
fi
