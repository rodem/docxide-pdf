"""Synthetic running-head DrawingML connectors, including zero-height lines.

Uses only the Python standard library; export input.docx with Microsoft Word
to regenerate reference.pdf. No external document or image assets.
"""
from pathlib import Path
import zipfile

W = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
R = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'
WP = 'http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing'
A = 'http://schemas.openxmlformats.org/drawingml/2006/main'
WPS = 'http://schemas.microsoft.com/office/word/2010/wordprocessingShape'
P = 'http://schemas.openxmlformats.org/package/2006/relationships'


def connector(ident, offset, height):
    return f'''<w:r><w:drawing><wp:anchor simplePos="0" relativeHeight="{ident}" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1">
<wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="column"><wp:posOffset>0</wp:posOffset></wp:positionH>
<wp:positionV relativeFrom="paragraph"><wp:posOffset>{round(offset*12700)}</wp:posOffset></wp:positionV>
<wp:extent cx="2540000" cy="{round(height*12700)}"/><wp:wrapNone/><wp:docPr id="{ident}" name="Synthetic line {ident}"/>
<a:graphic><a:graphicData uri="{WPS}"><wps:wsp><wps:cNvCnPr/><wps:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="2540000" cy="{round(height*12700)}"/></a:xfrm>
<a:prstGeom prst="straightConnector1"><a:avLst/></a:prstGeom><a:noFill/><a:ln w="25400"><a:solidFill><a:srgbClr val="24486C"/></a:solidFill></a:ln>
</wps:spPr><wps:bodyPr/></wps:wsp></a:graphicData></a:graphic></wp:anchor></w:drawing></w:r>'''


def generate(path):
    ns=f'xmlns:w="{W}" xmlns:r="{R}" xmlns:wp="{WP}" xmlns:a="{A}" xmlns:wps="{WPS}"'
    header=f'<w:hdr {ns}><w:p><w:pPr><w:rPr><w:sz w:val="20"/></w:rPr></w:pPr>{connector(1,13.2,0)}{connector(2,30,8)}</w:p></w:hdr>'
    doc=f'''<w:document {ns}><w:body><w:p><w:r><w:t>Running-head connector test, first page.</w:t></w:r></w:p>
<w:p><w:r><w:br w:type="page"/></w:r></w:p><w:p><w:r><w:t>Running-head connector test, second page.</w:t></w:r></w:p>
<w:sectPr><w:headerReference w:type="default" r:id="header"/><w:pgSz w:w="11906" w:h="16838"/>
<w:pgMar w:top="1440" w:bottom="1440" w:left="1440" w:right="1440" w:header="567" w:footer="567"/></w:sectPr></w:body></w:document>'''
    ct='''<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>'''
    parts={'[Content_Types].xml':ct,'word/document.xml':doc,'word/header1.xml':header,
           '_rels/.rels':f'<Relationships xmlns="{P}"><Relationship Id="document" Type="{R}/officeDocument" Target="word/document.xml"/></Relationships>',
           'word/_rels/document.xml.rels':f'<Relationships xmlns="{P}"><Relationship Id="header" Type="{R}/header" Target="header1.xml"/></Relationships>'}
    with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
        for name,xml in parts.items(): z.writestr(name,xml)



def grouped_fixture(path):
    generate(path)
    import xml.etree.ElementTree as ET
    with zipfile.ZipFile(path) as z: parts={n:z.read(n) for n in z.namelist()}
    ns=f'xmlns:w="{W}" xmlns:r="{R}" xmlns:wp="{WP}" xmlns:a="{A}" xmlns:wps="{WPS}" xmlns:wpg="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup"'
    # The child space is 200 x 16pt, stretched to 200 x 20pt and flipped.
    child1=connector(1,0,0).split('<wps:wsp>')[1].split('</wps:wsp>')[0]
    child2=connector(2,0,0).split('<wps:wsp>')[1].split('</wps:wsp>')[0]
    child1=child1.replace('<a:off x="0" y="0"/>','<a:off x="0" y="203200"/>')
    child1=child1.replace('w="25400"','w="44450"')
    group=f'<wpg:wgp><wpg:cNvGrpSpPr/><wpg:grpSpPr><a:xfrm flipV="1"><a:off x="0" y="0"/><a:ext cx="2540000" cy="254000"/><a:chOff x="0" y="0"/><a:chExt cx="2540000" cy="203200"/></a:xfrm></wpg:grpSpPr><wps:wsp>{child1}</wps:wsp><wps:wsp>{child2}</wps:wsp></wpg:wgp>'
    drawing=connector(3,10,20)
    begin=drawing.index('<a:graphicData'); end=drawing.index('</a:graphicData>')+len('</a:graphicData>')
    drawing=drawing[:begin]+f'<a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup">{group}</a:graphicData>'+drawing[end:]
    # Body rendering already paints connectors, so this fixture is independent
    # of the separate running-head connector fix.
    parts['word/document.xml']=f'<w:document {ns}><w:body><w:p>{drawing}<w:r><w:t>Reflected group connector test.</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:bottom="1440" w:left="1440" w:right="1440"/></w:sectPr></w:body></w:document>'
    with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
        for n,data in parts.items(): z.writestr(n,data)

if __name__=='__main__': grouped_fixture(Path(__file__).with_name('input.docx'))
