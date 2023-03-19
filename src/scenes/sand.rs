use rand::{prelude::SliceRandom, Rng};

use crate::{Canvas, FrameTick, Scene};

#[derive(Copy, Clone, PartialEq)]
struct Tile {
    type_: TileType,
    pressure: f32,
}

const EMPTY_TILE: Tile = Tile {
    type_: TileType::Empty,
    pressure: 0.0,
};

#[derive(Copy, Clone, PartialEq)]
enum TileType {
    Empty,
    Sand,
}

type Map = Vec<Vec<Tile>>;

trait MapTiles<T> {
    fn get_tile(&self, x: T, y: T) -> Option<Tile>;
    fn set_tile(&mut self, x: T, y: T, tile: Tile);
    fn in_bounds(&self, x: T, y: T) -> bool;
}

impl MapTiles<i32> for Map {
    fn get_tile(&self, x: i32, y: i32) -> Option<Tile> {
        if self.in_bounds(x, y) {
            Some(self[y as usize][x as usize])
        } else {
            None
        }
    }

    fn set_tile(&mut self, x: i32, y: i32, tile: Tile) {
        let row = self.get_mut(y as usize).unwrap();
        row[x as usize] = tile;
    }

    fn in_bounds(&self, x: i32, y: i32) -> bool {
        if x < (self.len() as i32) && y < (self[0].len() as i32) && x >= 0 && y >= 0 {
            true
        } else {
            false
        }
    }
}

pub struct SandScene {
    map: Map,

    last_spout: std::time::Instant,
}

impl SandScene {
    pub fn new(width: usize, height: usize) -> Self {
        let map: Map = vec![vec![EMPTY_TILE; width]; height];

        SandScene {
            map,
            last_spout: std::time::Instant::now(),
        }
    }

    fn draw(&self, canvas: &mut Canvas) {
        for y in 0..self.map.len() {
            for x in 0..self.map[y].len() {
                let tile = self.map[y][x];
                let p = (tile.pressure / 100.0).powf(2.0).clamp(0.0, 1.0);

                match tile.type_ {
                    TileType::Sand => {
                        canvas.set_pixel(x as u32, y as u32, 0.0, 0.9, 0.7);
                    }
                    _ => {
                        canvas.set_pixel(x as u32, y as u32, 0.0, 0.0, 0.0);
                    }
                }
            }
        }

        // let map = &self.map;

        // for y in 0..canvas.height {
        //     for x in 0..canvas.width {
        //         let index = (y * canvas.width + x) as usize;
        //         let value = map[index].powf(2.0);

        //         let hsv = Oklch::new(value.powf(1.0), 0.1, (value + t * 0.1) * 360.0);
        //         let rgb = Srgb::from_color(hsv);

        //         canvas.set_pixel(x, y, rgb.red, rgb.green, rgb.blue);
        //     }
        // }
    }
}

impl Scene for SandScene {
    fn tick(&mut self, canvas: &mut Canvas, _tick: &FrameTick) {
        let mut rng = rand::thread_rng();

        if self.last_spout.elapsed().as_secs_f32() >= 1.0 {
            if self.last_spout.elapsed().as_secs_f32() >= 2.0 {
                self.last_spout = std::time::Instant::now();
            }

            for _ in 0..5 {
                let x: i32 = rng.gen_range(-80..80) + 128;
                self.map[0][x as usize] = Tile {
                    type_: TileType::Sand,
                    pressure: 0.0,
                };
            }
        }

        let num_sand_tiles_in = self.map.iter().fold(0, |acc, row| {
            acc + row.iter().fold(0, |acc, tile| {
                if tile.type_ == TileType::Sand {
                    acc + 1
                } else {
                    acc
                }
            })
        });

        let mut to_update: Vec<(usize, usize)> = vec![];
        let hehe2 = self.map[1].len();
        let hehe = self.map[0].len();
        eprint!("{hehe2}");
        eprint!("{hehe}");
        for y in 0..self.map[0].len() {
            for x in 0..self.map[y].len() {
                to_update.push((x, y));
            }
        }

        to_update.shuffle(&mut rng);

        let mut updated: Vec<(i32, i32)> = vec![];

        for (x, y) in to_update.clone() {
            let ix = x as i32;
            let iy = y as i32;

            if updated.contains(&(ix, iy)) {
                continue;
            }

            let tile = self.map[y][x];
            if tile.type_ == TileType::Empty {
                continue;
            }

            // if iy == (self.map.len() - 1) as i32 && ix > 110 && ix < 120 {
            //     self.map.set_tile(ix, iy, EMPTY_TILE);
            // }

            let neighbors = [
                [
                    self.map.get_tile(ix - 1, iy - 1),
                    self.map.get_tile(ix, iy - 1),
                    self.map.get_tile(ix + 1, iy - 1),
                ],
                [
                    self.map.get_tile(ix - 1, iy),
                    self.map.get_tile(ix, iy),
                    self.map.get_tile(ix + 1, iy),
                ],
                [
                    self.map.get_tile(ix - 1, iy + 1),
                    self.map.get_tile(ix, iy + 1),
                    self.map.get_tile(ix + 1, iy + 1),
                ],
            ];

            if neighbors[2][1] == Some(EMPTY_TILE) {
                self.map.set_tile(ix, iy, EMPTY_TILE);
                self.map.set_tile(
                    ix,
                    iy + 1,
                    Tile {
                        pressure: 0.0,
                        ..tile
                    },
                );
                updated.push((ix, iy + 1));
            } else if neighbors[2][0] == Some(EMPTY_TILE) {
                self.map.set_tile(ix, iy, EMPTY_TILE);
                self.map.set_tile(
                    ix - 1,
                    iy + 1,
                    Tile {
                        pressure: 0.0,
                        ..tile
                    },
                );
                updated.push((ix - 1, iy + 1));
            } else if neighbors[2][2] == Some(EMPTY_TILE) {
                self.map.set_tile(ix, iy, EMPTY_TILE);
                self.map.set_tile(
                    ix + 1,
                    iy + 1,
                    Tile {
                        pressure: 0.0,
                        ..tile
                    },
                );
                updated.push((ix + 1, iy + 1));
            }
        }

        for (x, y) in to_update.clone() {
            let ix = x as i32;
            let iy = y as i32;

            let tile = self.map[y][x];
            if tile.type_ == TileType::Empty {
                continue;
            }

            let neighbors = [
                [
                    self.map.get_tile(ix - 1, iy - 1),
                    self.map.get_tile(ix, iy - 1),
                    self.map.get_tile(ix + 1, iy - 1),
                ],
                [
                    self.map.get_tile(ix - 1, iy),
                    self.map.get_tile(ix, iy),
                    self.map.get_tile(ix + 1, iy),
                ],
                [
                    self.map.get_tile(ix - 1, iy + 1),
                    self.map.get_tile(ix, iy + 1),
                    self.map.get_tile(ix + 1, iy + 1),
                ],
            ];

            if neighbors[0][1].is_some() && neighbors[0][1].unwrap().type_ == TileType::Sand {
                self.map[y][x].pressure = 100000.0 + neighbors[0][1].unwrap().pressure;
            }
        }

        for (x, y) in to_update.clone() {
            let ix = x as i32;
            let iy = y as i32;

            let tile = self.map[y][x];
            if tile.type_ == TileType::Empty {
                continue;
            }

            let neighbors = [
                [
                    self.map.get_tile(ix - 1, iy - 1),
                    self.map.get_tile(ix, iy - 1),
                    self.map.get_tile(ix + 1, iy - 1),
                ],
                [
                    self.map.get_tile(ix - 1, iy),
                    self.map.get_tile(ix, iy),
                    self.map.get_tile(ix + 1, iy),
                ],
                [
                    self.map.get_tile(ix - 1, iy + 1),
                    self.map.get_tile(ix, iy + 1),
                    self.map.get_tile(ix + 1, iy + 1),
                ],
            ];

            if tile.pressure > 0.1 {
                let pressure_over = tile.pressure - 0.1;

                if neighbors[1][0].is_some()
                    && neighbors[1][0].unwrap().type_ == TileType::Sand
                    && neighbors[1][2].is_some()
                    && neighbors[1][2].unwrap().type_ == TileType::Sand
                {
                    self.map[y][x - 1].pressure += pressure_over / 2.0;
                    self.map[y][x + 1].pressure += pressure_over / 2.0;
                } else if neighbors[1][0].is_some()
                    && neighbors[1][0].unwrap().type_ == TileType::Sand
                {
                    //if neighbors[1][0].unwrap().pressure > 0.1 {
                    self.map[y][x - 1].pressure += pressure_over;
                    //}
                } else if neighbors[1][2].is_some()
                    && neighbors[1][2].unwrap().type_ == TileType::Sand
                {
                    //if neighbors[1][2].unwrap().pressure > 0.1 {
                    self.map[y][x + 1].pressure += pressure_over;
                    //}
                }

                self.map[y][x].pressure -= pressure_over;
            }
        }

        updated.clear();

        for (x, y) in to_update.clone() {
            let ix = x as i32;
            let iy = y as i32;

            if updated.contains(&(ix, iy)) {
                continue;
            }

            let tile = self.map[y][x];
            if tile.type_ == TileType::Empty {
                continue;
            }

            let neighbors = [
                [
                    self.map.get_tile(ix - 1, iy - 1),
                    self.map.get_tile(ix, iy - 1),
                    self.map.get_tile(ix + 1, iy - 1),
                ],
                [
                    self.map.get_tile(ix - 1, iy),
                    self.map.get_tile(ix, iy),
                    self.map.get_tile(ix + 1, iy),
                ],
                [
                    self.map.get_tile(ix - 1, iy + 1),
                    self.map.get_tile(ix, iy + 1),
                    self.map.get_tile(ix + 1, iy + 1),
                ],
            ];

            if tile.pressure >= 0.1 {
                if neighbors[1][0] == Some(EMPTY_TILE) && neighbors[1][2] == Some(EMPTY_TILE) {
                    let either: bool = rng.gen();
                    if either {
                        self.map.set_tile(ix, iy, EMPTY_TILE);
                        self.map.set_tile(
                            ix - 1,
                            iy,
                            Tile {
                                pressure: tile.pressure - 0.1,
                                ..tile
                            },
                        );
                        updated.push((ix - 1, iy));
                    } else {
                        self.map.set_tile(ix, iy, EMPTY_TILE);
                        self.map.set_tile(
                            ix + 1,
                            iy,
                            Tile {
                                pressure: tile.pressure - 0.1,
                                ..tile
                            },
                        );
                        updated.push((ix + 1, iy));
                    }
                } else if neighbors[1][0] == Some(EMPTY_TILE) {
                    self.map.set_tile(ix, iy, EMPTY_TILE);
                    self.map.set_tile(
                        ix - 1,
                        iy,
                        Tile {
                            pressure: tile.pressure - 0.1,
                            ..tile
                        },
                    );
                    updated.push((ix - 1, iy));
                } else if neighbors[1][2] == Some(EMPTY_TILE) {
                    self.map.set_tile(ix, iy, EMPTY_TILE);
                    self.map.set_tile(
                        ix + 1,
                        iy,
                        Tile {
                            pressure: tile.pressure - 0.1,
                            ..tile
                        },
                    );
                    updated.push((ix + 1, iy));
                }
                // else if neighbors[0][1] == Some(EMPTY_TILE) {
                //     self.map.set_tile(ix, iy, EMPTY_TILE);
                //     self.map.set_tile(
                //         ix,
                //         iy - 1,
                //         Tile {
                //             pressure: tile.pressure - 0.1,
                //             ..tile
                //         },
                //     );
                //     updated.push((ix, iy - 1));
                // }
            }
        }

        let num_sand_tiles_out = self.map.iter().fold(0, |acc, row| {
            acc + row.iter().fold(0, |acc, tile| {
                if tile.type_ == TileType::Sand {
                    acc + 1
                } else {
                    acc
                }
            })
        });

        if num_sand_tiles_in != num_sand_tiles_out {
            println!(
                "in: {}\tout: {}\t{}",
                num_sand_tiles_in,
                num_sand_tiles_out,
                if num_sand_tiles_out > num_sand_tiles_in {
                    ">"
                } else {
                    "<="
                }
            );
        }

        self.draw(canvas);
    }
}
