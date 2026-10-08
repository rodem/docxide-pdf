"""Synthetic wrapped header pictures and bottom effect extent. Export with Word to regenerate reference.pdf."""
import io
import zipfile
from pathlib import Path
from PIL import Image
NS='xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:w10="urn:schemas-microsoft-com:office:word" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:w15="http://schemas.microsoft.com/office/word/2012/wordml" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"'
RELS='<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">\n<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>\n</Relationships>'

def drawing(rid, width, height, ident):
    w, h = round(width * 12700), round(height * 12700)
    return (f'<w:drawing><wp:inline><wp:extent cx="{w}" cy="{h}"/>'
            f'<wp:docPr id="{ident}" name="Synthetic {ident}"/>'
            '<a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">'
            '<a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">'
            '<pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">'
            f'<pic:nvPicPr><pic:cNvPr id="{ident}" name="Synthetic"/><pic:cNvPicPr/></pic:nvPicPr>'
            f'<pic:blipFill><a:blip r:embed="{rid}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>'
            f'<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{w}" cy="{h}"/></a:xfrm>'
            '<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr>'
            '</pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing>')

def fixture(path, font, size, effect=0):
    props = (f'<w:rPr><w:rFonts w:ascii="{font}" w:hAnsi="{font}"/>'
             f'<w:sz w:val="{size*2}"/></w:rPr>')
    header = ('<w:hdr ' + NS + '><w:p><w:pPr><w:spacing w:before="0" w:after="0" '
              'w:line="240" w:lineRule="auto"/>'+props+'</w:pPr>'
              + '<w:r>'+props+drawing('rId1',70,22,1)+'</w:r>'
              + '<w:r>'+props+drawing('rId2',468,4,2).replace('<wp:docPr',f'<wp:effectExtent l="0" t="0" r="0" b="{round(effect*12700)}"/><wp:docPr')+'</w:r></w:p>'
              + '<w:p><w:r>'+props+'<w:t>Header following marker</w:t></w:r></w:p></w:hdr>')
    body = ('<w:document '+NS+'><w:body><w:p><w:r><w:t>Body marker</w:t></w:r></w:p>'
            '<w:sectPr><w:headerReference w:type="default" r:id="header"/>'
            '<w:pgSz w:w="11906" w:h="16838"/>'
            '<w:pgMar w:top="1440" w:bottom="1440" w:left="1273" w:right="1273" '
            'w:header="567" w:footer="567"/></w:sectPr></w:body></w:document>')
    ct = ('<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
          '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
          '<Default Extension="png" ContentType="image/png"/>'
          '<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
          '<Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>')
    relns='http://schemas.openxmlformats.org/package/2006/relationships'
    reltype='http://schemas.openxmlformats.org/officeDocument/2006/relationships/'
    with zipfile.ZipFile(path,'w') as z:
        for name, value in [('[Content_Types].xml',ct),('_rels/.rels',RELS),
                            ('word/document.xml',body),('word/header1.xml',header),
                            ('word/_rels/document.xml.rels',f'<Relationships xmlns="{relns}"><Relationship Id="header" Type="{reltype}header" Target="header1.xml"/></Relationships>'),
                            ('word/_rels/header1.xml.rels',f'<Relationships xmlns="{relns}"><Relationship Id="rId1" Type="{reltype}image" Target="media/logo.png"/><Relationship Id="rId2" Type="{reltype}image" Target="media/stripe.png"/></Relationships>')]:
            z.writestr(name,value)
        for name,color in [('logo',(20,80,140)),('stripe',(140,30,20))]:
            buf=io.BytesIO(); Image.new('RGB',(100,30),color).save(buf,format='PNG')
            z.writestr('word/media/'+name+'.png',buf.getvalue())

if __name__ == "__main__":
    fixture(Path(__file__).with_name("input.docx"), "Times New Roman", 10, .75)
