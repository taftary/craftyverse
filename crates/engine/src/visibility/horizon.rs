//! Conservative horizon culling: chunks fully hidden by the planet body
//! relative to the camera are discarded.

use glam::Vec3;

use super::frustum::BoundingSphere;

/// The planet occlusion body for horizon culling: the surface sphere the
/// chunks live on. A chunk is culled only when it is certainly hidden
/// behind the planet limb from the camera; the test errs toward visible
/// in every marginal case (no popping at the horizon line):
///
/// - A camera at or below the surface radius culls nothing (near-ground
///   and inside-atmosphere flight keep the full loaded set).
/// - A sphere that pokes outside the tangent cone, however slightly,
///   stays visible (chunks straddling the limb are never culled).
/// - The radial test uses the tangent distance, the worst case over the
///   whole tangent cone, so a sphere beyond it is occluded along every
///   direction it spans.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanetHorizon {
    /// World-space planet center.
    pub origin: Vec3,
    /// Planet surface radius, in world units.
    pub radius: f32,
}

impl PlanetHorizon {
    /// The occlusion body of a planet with surface sphere
    /// (`origin`, `radius`).
    pub fn new(origin: Vec3, radius: f32) -> Self {
        PlanetHorizon { origin, radius }
    }

    /// Whether `sphere` is certainly hidden behind the planet limb from
    /// `camera`. Only fully-occluded spheres are culled; see the type
    /// docs for the conservativeness guarantees.
    ///
    /// The math: from a camera at distance `h > radius` from the planet
    /// center, the limb is the tangent cone toward the planet with
    /// half-angle `theta = asin(radius / h)`. A point is occluded iff its
    /// direction lies inside the cone and its distance from the camera
    /// exceeds the near sphere intersection along that ray (at most the
    /// tangent distance `sqrt(h^2 - radius^2)`). A sphere is culled when
    /// both hold for every point of it: its angular radius about the
    /// camera keeps it inside the cone, and its nearest point lies
    /// beyond the tangent distance.
    pub fn occludes(&self, camera: Vec3, sphere: &BoundingSphere) -> bool {
        let to_origin = self.origin - camera;
        let h = to_origin.length();
        if h <= self.radius {
            // Camera at or below the surface: the tangent cone does not
            // exist, everything is potentially visible.
            return false;
        }
        let to_center = sphere.center - camera;
        let d = to_center.length();
        if d <= sphere.radius {
            // The camera is inside the bounding sphere.
            return false;
        }
        // Cone containment: the sphere's angular radius `alpha` (as seen
        // from the camera) added to its center angle `phi` off the cone
        // axis must stay within `theta`, i.e. cos(phi + alpha) >=
        // cos(theta). All angles are in [0, pi] here (alpha < pi/2
        // because d > radius, phi <= pi by definition), and cos is
        // monotone decreasing there, so the cosine comparison is exact.
        let sin_theta = self.radius / h;
        let cos_theta = (1.0 - sin_theta * sin_theta).sqrt();
        let cos_phi = (to_center.dot(to_origin) / (d * h)).clamp(-1.0, 1.0);
        let sin_phi = (1.0 - cos_phi * cos_phi).sqrt();
        let sin_alpha = sphere.radius / d;
        let cos_alpha = (1.0 - sin_alpha * sin_alpha).sqrt();
        let cos_farthest_edge = cos_phi * cos_alpha - sin_phi * sin_alpha;
        if cos_farthest_edge < cos_theta {
            // The sphere pokes outside the tangent cone: part of it is
            // beside (not behind) the planet.
            return false;
        }
        // Radial: the sphere's nearest point must lie beyond the tangent
        // distance, the maximum near-intersection distance over the cone.
        let tangent_distance = (h * h - self.radius * self.radius).sqrt();
        d - sphere.radius > tangent_distance
    }
}
