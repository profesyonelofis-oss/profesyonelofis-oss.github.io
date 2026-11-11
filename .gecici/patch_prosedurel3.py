from pathlib import Path

p = Path('src/demo_site.rs')
s = p.read_text(encoding='utf-8')

eski_hesap = '    let dunya = dunya_uret(&tohum, &vurgu, &zemin);\n'
yeni_hesap = eski_hesap + (
    '    let dunya_stil = format!(\n'
    '        "--sn-akis1:{};--sn-akis2:{};--sn-hiz:{:.2};--sn-dalga:{:.2};--sn-parca:{:.2};'
    '--sn-vinyet:{:.2};--sn-aci:{};--sn-aci2:{}",\n'
    '        dunya.akis1, dunya.akis2, dunya.hiz, dunya.dalga, dunya.parca, dunya.vinyet,\n'
    '        dunya.aci, dunya.aci2\n'
    '    );\n'
)
assert eski_hesap in s
s = s.replace(eski_hesap, yeni_hesap, 1)

eski_stil = ('style="--sn-akis1:{akis1};--sn-akis2:{akis2};--sn-hiz:{hiz};--sn-dalga:{dalga};'
             '--sn-parca:{parca};--sn-vinyet:{vinyet};--sn-aci:{aci};--sn-aci2:{aci2}"')
assert eski_stil in s
s = s.replace(eski_stil, 'style="{dunya_stil}"', 1)

eski_arg = '''        dunya_tohum = dunya.tohum,
        akis1 = dunya.akis1,
        akis2 = dunya.akis2,
        hiz = format!("{:.2}", dunya.hiz),
        dalga = format!("{:.2}", dunya.dalga),
        parca = format!("{:.2}", dunya.parca),
        vinyet = format!("{:.2}", dunya.vinyet),
        aci = dunya.aci,
        aci2 = dunya.aci2,
'''
assert eski_arg in s
s = s.replace(eski_arg, '        dunya_tohum = dunya.tohum,\n        dunya_stil = dunya_stil,\n', 1)
p.write_text(s, encoding='utf-8')
print('dunya_stil hazır')
