#!/usr/bin/env python3
# SPDX-License-Identifier: BSD-3-Clause

from pathlib    import Path
from urllib     import request
from zipfile    import ZipFile
from itertools  import product
from subprocess import run

SRC_ROOT   = Path(__file__).resolve().parent.parent
BUILD_DIR  = SRC_ROOT / 'target'

FONT_CRATE      = SRC_ROOT / 'iosevka-embedded'
FONT_RASTER_DIR = FONT_CRATE / 'fonts'
FONT_CACHE      = BUILD_DIR / 'iosevka'

FONT_VERSION = '33.2.1'
FONT_ARCHIVE = f'PkgTTF-IosevkaFixed-{FONT_VERSION}.zip'
FONT_URL = f'https://github.com/be5invis/Iosevka/releases/download/v{FONT_VERSION}/{FONT_ARCHIVE}'

FONT_BASE   = 'IosevkaFixed-Extended'
FONT_SIZES  = (8, 16, 24, 32, )
FONT_STYLES = ('', 'Bold', 'Thin')

FONT_SETS = list(map(lambda s: (f'{FONT_BASE}{s[0]}', s[1]), product(FONT_STYLES, FONT_SIZES,)))

def main() -> int:
	FONT_CACHE.mkdir(parents = True, exist_ok = True)
	FONT_RASTER_DIR.mkdir(parents = True, exist_ok = True)

	if not (FONT_CACHE / 'IosevkaFixed-Extended.ttf').exists():
		print('Fonts not found, downloading')
		font_archive, _ = request.urlretrieve(FONT_URL)
		print('Extracting font archive')
		with ZipFile(font_archive) as f:
			f.extractall(FONT_CACHE)

	print(f'Building {len(FONT_SETS)} font permutations...')
	for font_name, font_size in FONT_SETS:
		font_file = FONT_CACHE / f'{font_name}.ttf'
		bdf_file  = FONT_RASTER_DIR / f'{font_name}.bdf'

		if not font_file.exists():
			print(f'ERROR: Unable to find font file matching {font_name}, skipping...')
			continue

		if bdf_file.exists():
			print('Rasterized bitmap font already exists, skipping...')
			continue

		print(f'Generating {bdf_file}...')

		run(f'fontforge -lang=ff -c \'Open("{str(font_file)}"); BitmapsAvail([{font_size}]); BitmapsRegen([{font_size}]); Generate("{FONT_RASTER_DIR}/{font_file.stem}.", "bdf")\'', cwd = Path.cwd(), capture_output=True, shell=True)


	return 0

if __name__ == '__main__':
	raise SystemExit(main())
