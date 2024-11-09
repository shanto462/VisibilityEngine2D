using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Media;
using System.Windows.Shapes;

namespace VisibilityIn2DGrid.Culling;

public class OcclusionCuller
{
    public class VisibilityRay(Point start, Point end, bool isBlocked)
    {
        public Point Start { get; set; } = start;
        public Point End { get; set; } = end;
        public bool IsBlocked { get; set; } = isBlocked;
    }

    public class ViewOcclusion
    {
        public Point Center { get; }
        public double FOVAngle { get; }
        public double Direction { get; }
        public double Range { get; }

        public ViewOcclusion(Point center, double fovAngle, double direction, double range)
        {
            Center = center;
            FOVAngle = fovAngle;
            Direction = direction;
            Range = range;
        }

        public (Point, Point, Point) GetViewLines()
        {
            double startAngle = (Direction - FOVAngle / 2) * Math.PI / 180;
            double endAngle = (Direction + FOVAngle / 2) * Math.PI / 180;

            Point leftPoint = new(
                Center.X + Range * Math.Cos(startAngle),
                Center.Y + Range * Math.Sin(startAngle)
            );

            Point rightPoint = new(
                Center.X + Range * Math.Cos(endAngle),
                Center.Y + Range * Math.Sin(endAngle)
            );

            return (Center, leftPoint, rightPoint);
        }

        public bool IsPointInView(Point point)
        {
            double distanceToCenter = Math.Sqrt(
                Math.Pow(point.X - Center.X, 2) +
                Math.Pow(point.Y - Center.Y, 2)
            );

            if (distanceToCenter > Range)
                return false;

            double angleToPoint = Math.Atan2(
                point.Y - Center.Y,
                point.X - Center.X
            ) * 180 / Math.PI;

            double normalizedDirection = Direction % 360;
            if (normalizedDirection < 0) normalizedDirection += 360;

            double normalizedAngle = angleToPoint % 360;
            if (normalizedAngle < 0) normalizedAngle += 360;

            double halfFOV = FOVAngle / 2;
            double angleDiff = Math.Abs(normalizedAngle - normalizedDirection);
            if (angleDiff > 180) angleDiff = 360 - angleDiff;

            return angleDiff <= halfFOV;
        }
    }

    private readonly Dictionary<Polygon, double> _polygonDistances = new();

    public (List<Polygon> visiblePolygons, Polygon viewArea, List<VisibilityRay> rays) CalculateVisibility(
        Point center,
        double fovAngle,
        double direction,
        double range,
        IEnumerable<Polygon> polygons,
        double step = 0.5)
    {
        var view = new ViewOcclusion(center, fovAngle, direction, range);
        var (viewCenter, viewLeft, viewRight) = view.GetViewLines();

        // Create view area polygon
        var viewAreaPoints = new PointCollection();
        viewAreaPoints.Add(viewCenter);
        viewAreaPoints.Add(viewLeft);

        // Add arc points
        double startAngle = (direction - fovAngle / 2) * Math.PI / 180;
        double endAngle = (direction + fovAngle / 2) * Math.PI / 180;

        if (startAngle > endAngle)
        {
            endAngle += 2 * Math.PI;
        }

        for (double angle = startAngle; angle <= endAngle; angle += step * Math.PI / 180)
        {
            viewAreaPoints.Add(new Point(
                center.X + range * Math.Cos(angle),
                center.Y + range * Math.Sin(angle)
            ));
        }

        viewAreaPoints.Add(viewRight);

        var viewAreaPolygon = new Polygon
        {
            Points = viewAreaPoints,
            Fill = new SolidColorBrush(Color.FromArgb(90, 255, 255, 0)),
            Stroke = new SolidColorBrush(Colors.Yellow),
            StrokeThickness = 2
        };

        // Calculate distances for all polygons
        _polygonDistances.Clear();
        foreach (var polygon in polygons)
        {
            if (IsPolygonInView(polygon, view))
            {
                double distance = CalculatePolygonDistance(polygon, center);
                _polygonDistances[polygon] = distance;
            }
        }

        // Sort polygons by distance and check occlusion
        var sortedPolygons = _polygonDistances
            .OrderBy(kvp => kvp.Value)
            .Select(kvp => kvp.Key)
            .ToList();

        var rays = new List<VisibilityRay>();

        var visiblePolygons = new List<Polygon>();

        foreach (var polygon in sortedPolygons)
        {
            bool isVisible = false;
            foreach (var point in polygon.Points)
            {
                if (IsPointVisible(point, center, visiblePolygons, out bool isBlocked))
                {
                    isVisible = true;
                    rays.Add(new VisibilityRay(center, point, isBlocked));
                }
            }
            if (isVisible)
            {
                visiblePolygons.Add(polygon);
            }
        }

        return (visiblePolygons, viewAreaPolygon, rays);
    }

    private bool IsPointVisible(Point point, Point viewPoint, List<Polygon> occluders, out bool isBlocked)
    {
        isBlocked = false;
        var ray = point - viewPoint;
        double rayLength = ray.Length;
        double distanceToPoint = rayLength;

        foreach (var occluder in occluders)
        {
            double occluderDistance = _polygonDistances[occluder];

            if (occluderDistance >= distanceToPoint)
                continue;

            if (DoesPolygonBlockRay(occluder, viewPoint, ray, rayLength))
            {
                isBlocked = true;
                return false;
            }
        }

        return true;
    }

    private static bool IsPolygonInView(Polygon polygon, ViewOcclusion view)
    {
        foreach (var point in polygon.Points)
        {
            if (view.IsPointInView(point))
                return true;
        }
        return false;
    }

    private double CalculatePolygonDistance(Polygon polygon, Point center)
    {
        return polygon.Points.Min(point =>
            Math.Sqrt(Math.Pow(point.X - center.X, 2) + Math.Pow(point.Y - center.Y, 2))
        );
    }

    private bool IsPolygonVisible(Polygon target, Point viewPoint, List<Polygon> occluders)
    {
        // Check if any point of the target is visible
        foreach (var point in target.Points)
        {
            if (IsPointVisible(point, viewPoint, occluders))
                return true;
        }
        return false;
    }

    private bool IsPointVisible(Point point, Point viewPoint, List<Polygon> occluders)
    {
        var ray = point - viewPoint;
        double rayLength = ray.Length;
        double distanceToPoint = rayLength;

        foreach (var occluder in occluders)
        {
            double occluderDistance = _polygonDistances[occluder];

            // Skip occluders that are farther than the point we're checking
            if (occluderDistance >= distanceToPoint)
                continue;

            if (DoesPolygonBlockRay(occluder, viewPoint, ray, rayLength))
                return false;
        }

        return true;
    }

    private bool DoesPolygonBlockRay(Polygon polygon, Point rayOrigin, Vector rayDirection, double rayLength)
    {
        for (int i = 0; i < polygon.Points.Count; i++)
        {
            var p1 = polygon.Points[i];
            var p2 = polygon.Points[(i + 1) % polygon.Points.Count];

            if (RayIntersectsLine(rayOrigin, rayDirection, p1, p2, out double intersectionDistance))
            {
                if (intersectionDistance > 0 && intersectionDistance < rayLength)
                    return true;
            }
        }
        return false;
    }

    private bool RayIntersectsLine(Point rayOrigin, Vector rayDir, Point lineStart, Point lineEnd, out double intersectionDistance)
    {
        intersectionDistance = 0;

        var v1 = rayOrigin - lineStart;
        var v2 = lineEnd - lineStart;
        var v3 = new Vector(-rayDir.Y, rayDir.X);

        double dot = Vector.Multiply(v2, v3);
        if (Math.Abs(dot) < 0.000001)
            return false;

        double t1 = Vector.CrossProduct(v2, v1) / dot;
        double t2 = Vector.Multiply(v1, v3) / dot;

        if (t1 >= 0.0 && (t2 >= 0.0 && t2 <= 1.0))
        {
            intersectionDistance = t1;
            return true;
        }

        return false;
    }
}
