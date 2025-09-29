use crate::*;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
type Vertex = StandardVertex;
type Result<T> = std::result::Result<T, errors::Error>;

pub fn write<W: Write>(mesh: &PolygonMesh, writer: W) -> Result<()> {
    sub_write(mesh, &mut BufWriter::new(writer))
}

pub fn write_vec<W: Write>(mesh: &[PolygonMesh], writer: W) -> Result<()> {
    let mut writer = BufWriter::new(writer);
    for (i, mesh) in mesh.iter().enumerate() {
        writer.write_fmt(format_args!("g {i}\n"))?;
        sub_write(mesh, &mut writer)?;
    }
    Ok(())
}
fn write2vec<V: std::ops::Index<usize, Output = f64>, W: Write>(
    writer: &mut BufWriter<W>,
    vecs: &[V],
    prefix: &str,
) -> Result<()> {
    for vec in vecs {
        writer.write_fmt(format_args!("{} {:.10e} {:.10e}\n", prefix, vec[0], vec[1]))?;
    }
    Ok(())
}

fn write3vec<V: std::ops::Index<usize, Output = f64>, W: Write>(
    writer: &mut BufWriter<W>,
    vecs: &[V],
    prefix: &str,
) -> Result<()> {
    for vec in vecs {
        writer.write_fmt(format_args!(
            "{} {:.10e} {:.10e} {:.10e}\n",
            prefix, vec[0], vec[1], vec[2]
        ))?;
    }
    Ok(())
}

impl Vertex {
    fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match (self.uv, self.nor) {
            (None, None) => writer.write_fmt(format_args!("{}", self.pos + 1)),
            (Some(uv), None) => writer.write_fmt(format_args!("{}/{}", self.pos + 1, uv + 1)),
            (None, Some(nor)) => writer.write_fmt(format_args!("{}//{}", self.pos + 1, nor + 1)),
            (Some(uv), Some(nor)) => {
                writer.write_fmt(format_args!("{}/{}/{}", self.pos + 1, uv + 1, nor + 1))
            }
        }
    }
}

impl Faces {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        for face in self.face_iter() {
            writer.write_all(b"f")?;
            for v in face {
                writer.write_all(b" ")?;
                v.write(writer)?;
            }
            writer.write_all(b"\n")?;
        }
        Ok(())
    }
}

fn sub_write<W: Write>(mesh: &PolygonMesh, writer: &mut BufWriter<W>) -> Result<()> {
    write3vec(writer, mesh.positions(), "v")?;
    write2vec(writer, mesh.uv_coords(), "vt")?;
    write3vec(writer, mesh.normals(), "vn")?;
    mesh.faces.write(writer)
}

pub fn read<R: Read>(reader: R) -> Result<PolygonMesh> {
    let mut positions = Vec::new();
    let mut uv_coords = Vec::new();
    let mut normals = Vec::new();
    let mut faces = Faces::default();
    let reader = BufReader::new(reader);
    for line in reader.lines().map(|s| s.unwrap()) {
        let mut args = line.split_whitespace();
        if let Some(first_str) = args.next() {
            if first_str == "v" {
                let x = args.next().unwrap().parse::<f64>()?;
                let y = args.next().unwrap().parse::<f64>()?;
                let z = args.next().unwrap().parse::<f64>()?;
                positions.push(Point3::new(x, y, z));
            } else if first_str == "vt" {
                let u = args.next().unwrap().parse::<f64>()?;
                let v = args.next().unwrap().parse::<f64>()?;
                uv_coords.push(Vector2::new(u, v));
            } else if first_str == "vn" {
                let x = args.next().unwrap().parse::<f64>()?;
                let y = args.next().unwrap().parse::<f64>()?;
                let z = args.next().unwrap().parse::<f64>()?;
                normals.push(Vector3::new(x, y, z));
            } else if first_str == "f" {
                let mut face = Vec::new();
                for vert_str in args {
                    if &vert_str[0..1] == "#" {
                        break;
                    }
                    let mut iter = vert_str.split('/');
                    let pos = iter
                        .next()
                        .map(|val| val.parse::<usize>().map(|i| i - 1).ok())
                        .unwrap_or(None);
                    let uv = iter
                        .next()
                        .map(|val| val.parse::<usize>().map(|i| i - 1).ok())
                        .unwrap_or(None);
                    let nor = iter
                        .next()
                        .map(|val| val.parse::<usize>().map(|i| i - 1).ok())
                        .unwrap_or(None);
                    let vert = match (pos, uv, nor) {
                        (None, _, _) => continue,
                        (Some(pos), uv, nor) => Vertex { pos, uv, nor },
                    };
                    face.push(vert);
                }
                faces.push(face);
            }
        }
    }
    PolygonMesh::try_new(
        StandardAttributes {
            positions,
            uv_coords,
            normals,
        },
        faces,
    )
}
