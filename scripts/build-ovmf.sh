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

EDK2_GIT="${TARGET_DIR}/edk2.git"
EDK2_REPO="https://github.com/tianocore/edk2.git"
EDK2_REV="edk2-stable202408.01"
EDK2_VENV="${TARGET_DIR}/edk2.venv"

OVMF_DIR="${TARGET_DIR}/.ovmf"
OVMF_IMG_DIR="${TARGET_DIR}/.ovmf/efi"
OVMF_BLD="${EDK2_GIT}/Build/OvmfX64/DEBUG_GCC"

[ ! -d "${OVMF_DIR}" ] && mkdir "${OVMF_DIR}"
[ ! -d "${OVMF_IMG_DIR}" ] && mkdir "${OVMF_IMG_DIR}"

pushd "${TARGET_DIR}" || exit

_efi_args=(
	-D FD_SIZE_4MB
	-D NETWORK_HTTP_BOOT_ENABLE
	-D NETWORK_IP6_ENABLE
	-D TPM_CONFIG_ENABLE
	-D TPM_ENABLE
	-D TPM1_ENABLE
	-D TPM2_ENABLE
)


# Clone the EDKII repo
if [ ! -d "${EDK2_GIT}" ]; then
	git clone "${EDK2_REPO}" "${EDK2_GIT}"
	pushd "${EDK2_GIT}" || exit
	git checkout "${EDK2_REV}"
	git submodule update --init
	CC=gcc-13 make -C BaseTools
	popd || exit
fi

if [ ! -f "${OVMF_BLD}/FV/OVMF_CODE.fd" ]; then
	pushd "${EDK2_GIT}" || exit
	source edksetup.sh
	BaseTools/BinWrappers/PosixLike/build -p OvmfPkg/OvmfPkgX64.dsc -a X64 -b DEBUG -t GCC "${_efi_args[@]}"
	popd || exit
fi

[ ! -f "${OVMF_DIR}/OVMF_CODE.4m.fd" ] && cp "${OVMF_BLD}/FV/OVMF_CODE.fd" "${OVMF_DIR}/OVMF_CODE.4m.fd"
[ ! -f "${OVMF_DIR}/OVMF_VARS.4m.fd" ] && cp "${OVMF_BLD}/FV/OVMF_VARS.fd" "${OVMF_DIR}/OVMF_VARS.4m.fd"

pushd "${OVMF_BLD}/X64" || exit

for efi_img in *.efi; do
	[ ! -f "${OVMF_IMG_DIR}/${efi_img}" ] && cp "${OVMF_BLD}/X64/${efi_img}" "${OVMF_IMG_DIR}/${efi_img}"
done
for efi_dbg in *.debug; do
	[ ! -f "${OVMF_IMG_DIR}/${efi_dbg}" ] && cp "${OVMF_BLD}/X64/${efi_dbg}" "${OVMF_IMG_DIR}/${efi_dbg}"
done
popd || exit



popd || exit
