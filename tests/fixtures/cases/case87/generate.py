"""A continuous section starts on an empty page and uses its first header."""
from pathlib import Path
import zipfile
W='http://schemas.openxmlformats.org/wordprocessingml/2006/main'
R='http://schemas.openxmlformats.org/officeDocument/2006/relationships'
P='http://schemas.openxmlformats.org/package/2006/relationships'

def paragraph(text):
    return '<w:p><w:r><w:t>'+text+'</w:t></w:r></w:p>'

def section(continuous=False):
    refs='' if continuous else '<w:headerReference w:type="default" r:id="default"/><w:headerReference w:type="first" r:id="first"/>'
    return '<w:sectPr>'+refs+('<w:type w:val="continuous"/>' if continuous else '')+'<w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:bottom="1440" w:left="1440" w:right="1440" w:header="567" w:footer="567"/><w:titlePg/></w:sectPr>'

def generate(path):
    ns=f'xmlns:w="{W}" xmlns:r="{R}"'
    br='<w:p><w:r><w:br w:type="page"/></w:r></w:p>'
    body=paragraph('First section first page.')+br+paragraph('First section second page.')
    body+=br+'<w:p><w:pPr>'+section()+'</w:pPr></w:p>'
    body+=paragraph('Continuous section begins on this otherwise empty third page.')+section(True)
    ct='<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
    for name in ['default','first']: ct+=f'<Override PartName="/word/{name}.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/>'
    ct+='</Types>'
    rels=f'<Relationships xmlns="{P}">'+''.join(f'<Relationship Id="{name}" Type="{R}/header" Target="{name}.xml"/>' for name in ['default','first'])+'</Relationships>'
    parts={'[Content_Types].xml':ct,'_rels/.rels':f'<Relationships xmlns="{P}"><Relationship Id="doc" Type="{R}/officeDocument" Target="word/document.xml"/></Relationships>','word/document.xml':f'<w:document {ns}><w:body>{body}</w:body></w:document>','word/_rels/document.xml.rels':rels}
    for name in ['default','first']: parts['word/'+name+'.xml']=f'<w:hdr {ns}>'+paragraph(name.upper()+' HEADER MARKER')+'</w:hdr>'
    with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
        for n,xml in parts.items(): z.writestr(n,xml)

if __name__=='__main__': generate(Path(__file__).with_name('input.docx'))
