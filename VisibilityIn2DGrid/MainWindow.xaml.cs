﻿using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Shapes;
using VisibilityIn2DGrid.Attributes;
using VisibilityIn2DGrid.Culling;
using VisibilityIn2DGrid.Extensions;
using VisibilityIn2DGrid.Helper;
using VisibilityIn2DGrid.Index;
using VisibilityIn2DGrid.RayTracing;

namespace VisibilityIn2DGrid;

public partial class MainWindow : Window
{

    private const int GridSize = 50;
    private const double GridLineThickness = 0.5;

    private const double CanvasWidth = 2000;
    private const double CanvasHeight = 2000;

    private const double CenterMarkerSize = 20;
    private const double CrossThickness = 2;

    private readonly Random _random = new(Guid.NewGuid().GetHashCode());

    private const double ZoomMin = 0.1;
    private const double ZoomMax = 5.0;
    private const double ZoomSpeed = 1.1;
    private double _currentZoom = 1.0;
    private Point? _lastMousePosition;
    private bool _isDragging = false;

    private readonly float _visibilityRange = 600.0f;

    private readonly List<Polygon> obstacles = [];

    private SpatialIndex? _spatialIndex;

    private Point _center;
    private Polygon? _visibilityPolygon;

    private Polygon? _frustumPolygon;
    private List<(Polygon, Brush)> _frustumPolygons = [];

    private float _frustumFOVAngle = 90.0f;
    private float _frustumDirection = 0.0f;

    private const float _keyboardChangeStep = 5.0f;

    private DateTime lastClickTime = DateTime.MinValue;
    private const double DoubleClickTimeMs = 300;

    private readonly Ellipse centerCircle = new()
    {
        Width = CenterMarkerSize,
        Height = CenterMarkerSize,
        Fill = new SolidColorBrush(Color.FromArgb(50, 0, 150, 255)),
        Stroke = new SolidColorBrush(Color.FromArgb(200, 0, 100, 200)),
        StrokeThickness = 2
    };

    public MainWindow()
    {
        InitializeComponent();

        canvas.Width = CanvasWidth;
        canvas.Height = CanvasHeight;

        canvas.MouseLeftButtonDown += canvas_MouseLeftButtonDown;
        canvas.MouseLeftButtonUp += canvas_MouseLeftButtonUp;
        canvas.MouseMove += canvas_MouseMove;
        canvas.MouseMove += UpdateMousePosition;

        this.KeyDown += MainWindow_KeyDown;
        this.Loaded += MainWindow_Loaded;
        this.SizeChanged += MainWindow_SizeChanged;
    }

    private void MainWindow_KeyDown(object sender, KeyEventArgs e)
    {
        if (!canvas.IsMouseOverCanvasContent(_scrollViewer))
            return;

        if (fcRB.IsChecked == true)
        {
            bool reload = false;

            if (Keyboard.IsKeyDown(Key.LeftAlt) && Keyboard.IsKeyDown(Key.Up))
            {
                reload = true;

                _frustumFOVAngle += _keyboardChangeStep;
            }
            else if (Keyboard.IsKeyDown(Key.LeftAlt) && Keyboard.IsKeyDown(Key.Down))
            {
                reload = true;

                _frustumFOVAngle -= _keyboardChangeStep;
            }
            else if (Keyboard.IsKeyDown(Key.LeftAlt) && Keyboard.IsKeyDown(Key.Left))
            {
                reload = true;

                _frustumDirection += _keyboardChangeStep;
            }
            else if (Keyboard.IsKeyDown(Key.LeftAlt) && Keyboard.IsKeyDown(Key.Right))
            {
                reload = true;

                _frustumDirection -= _keyboardChangeStep;
            }

            if (_frustumFOVAngle < 0.0f)
                _frustumFOVAngle = 0.0f;
            else if (_frustumFOVAngle > 360.0f)
                _frustumFOVAngle = 360.0f;

            if (reload)
            {
                ExecuteAlgorithm();
            }
        }

        e.Handled = true;
    }

    private void UpdateMousePosition(object sender, MouseEventArgs e)
    {
        Point mousePos = e.GetPosition(canvas);
        MousePositionText.Text = $"Position: {mousePos.X:F0}, {mousePos.Y:F0}";
    }

    private void canvas_MouseLeftButtonDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Left)
        {
            var clickTime = DateTime.Now;
            if ((clickTime - lastClickTime).TotalMilliseconds <= DoubleClickTimeMs)
            {
                var pos = Mouse.GetPosition(canvas);

                _center = pos;

                TransformCenter();
                ExecuteAlgorithm();

                lastClickTime = DateTime.MinValue; // Reset timer
            }
            else
            {
                lastClickTime = clickTime;

                _isDragging = true;
                _lastMousePosition = e.GetPosition(_scrollViewer);
                canvas.Cursor = Cursors.Hand;
                ((UIElement)sender).CaptureMouse();
            }
        }
    }

    private void canvas_MouseLeftButtonUp(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Left)
        {
            _isDragging = false;
            _lastMousePosition = null;
            canvas.Cursor = Cursors.Arrow;
            ((UIElement)sender).ReleaseMouseCapture();
        }
    }

    private void canvas_MouseMove(object sender, MouseEventArgs e)
    {
        if (_isDragging && _lastMousePosition.HasValue)
        {
            Point currentPosition = e.GetPosition(_scrollViewer);
            double deltaX = currentPosition.X - _lastMousePosition.Value.X;
            double deltaY = currentPosition.Y - _lastMousePosition.Value.Y;

            _scrollViewer.ScrollToHorizontalOffset(_scrollViewer.HorizontalOffset - deltaX);
            _scrollViewer.ScrollToVerticalOffset(_scrollViewer.VerticalOffset - deltaY);

            _lastMousePosition = currentPosition;
        }
    }

    private void ScrollViewer_PreviewMouseWheel(object sender, MouseWheelEventArgs e)
    {
        if (Keyboard.Modifiers == ModifierKeys.Control)
        {
            Point mousePos = e.GetPosition(canvas);

            if (e.Delta > 0)
                ZoomIn(mousePos);
            else
                ZoomOut(mousePos);

            e.Handled = true;
        }
    }

    private void ZoomIn(Point center)
    {
        AdjustZoom(ZoomSpeed, center);
    }

    private void ZoomOut(Point center)
    {
        AdjustZoom(1 / ZoomSpeed, center);
    }

    private void AdjustZoom(double factor, Point center)
    {
        double newZoom = _currentZoom * factor;

        newZoom = Math.Max(ZoomMin, Math.Min(ZoomMax, newZoom));

        if (Math.Abs(newZoom - _currentZoom) < 0.001)
            return;

        _currentZoom = newZoom;

        ZoomTransform.ScaleX = _currentZoom;
        ZoomTransform.ScaleY = _currentZoom;

        ZoomLevelText.Text = $"{_currentZoom:P0}";
    }

    private void ZoomInButton_Click(object sender, RoutedEventArgs e)
    {
        Point center = new(canvas.ActualWidth / 2, canvas.ActualHeight / 2);
        ZoomIn(center);
    }

    private void ZoomOutButton_Click(object sender, RoutedEventArgs e)
    {
        Point center = new(canvas.ActualWidth / 2, canvas.ActualHeight / 2);
        ZoomOut(center);
    }

    private void ResetZoomButton_Click(object sender, RoutedEventArgs e)
    {
        _currentZoom = 1.0;
        ZoomTransform.ScaleX = 1.0;
        ZoomTransform.ScaleY = 1.0;
        ZoomLevelText.Text = "100%";

        CenterCanvas();
    }

    private void MainWindow_Loaded(object sender, RoutedEventArgs e)
    {
        CenterCanvas();

        double x = canvas.Width / 2;
        double y = canvas.Height / 2;

        _center = new Point(x, y);

        CanvasSizeText.Text = $"Canvas Size: {CanvasWidth} x {CanvasHeight}";

        DrawCenter();
        RefreshScreen();
    }

    private void MainWindow_SizeChanged(object sender, SizeChangedEventArgs e)
    {
        CenterCanvas();
    }

    private void CenterCanvas()
    {
        if (_scrollViewer == null) return;

        GetCanvasCenter(out double horizontalOffset, out double verticalOffset);

        _scrollViewer.ScrollToHorizontalOffset(horizontalOffset);
        _scrollViewer.ScrollToVerticalOffset(verticalOffset);
    }

    private void GetCanvasCenter(out double horizontalOffset, out double verticalOffset)
    {
        horizontalOffset = (CanvasWidth - _scrollViewer.ViewportWidth) / 2;
        verticalOffset = (CanvasHeight - _scrollViewer.ViewportHeight) / 2;
    }

    private void DrawGrid()
    {
        GridLineGenerator.DrawMajorGridLines(canvas, CanvasWidth, CanvasHeight, GridSize, GridLineThickness);
    }

    private void Button_Click(object sender, RoutedEventArgs e)
    {
        RefreshScreen();
    }

    private void RefreshScreen()
    {
        this.RunWithProgressBar(async () =>
        {
            await Task.Delay(100);

            this.RunOnUIThread(() =>
            {
                DrawGrid();

                this.DoEvents();

                foreach (var child in obstacles)
                {
                    canvas.Children.Remove(child);
                }

                _spatialIndex?.Dispose();
                _spatialIndex = new SpatialIndex((float)CanvasWidth, (float)CanvasHeight, 50.0f);

                AddRandomPolygons();

                this.DoEvents();

                CalculateSpatialIndex();

                ExecuteAlgorithm();

                TransformCenter();

                this.DoEvents();
            });

            await Task.Delay(500);
        });
    }

    private void TransformCenter()
    {
        Canvas.SetLeft(centerCircle, _center.X - CenterMarkerSize / 2);
        Canvas.SetTop(centerCircle, _center.Y - CenterMarkerSize / 2);
    }

    private void DrawCenter()
    {
        canvas.Children.Add(centerCircle);
    }

    private void CalculateVisibilty()
    {
        RemoveViews();

        double x = _center.X;
        double y = _center.Y;

        var obstacles = _spatialIndex?.QueryBounds(
            new Point(x, y),
            _visibilityRange
        )?.ToList();

        using var shadowCast2D = new ShadowCast2D(new Point(x, y), _visibilityRange);

        foreach (var obstacle in obstacles ?? [])
        {
            shadowCast2D.AddPolygon(obstacle);
        }

        var polygon = shadowCast2D.ComputeVisibility();

        canvas.Children.Add(polygon);

        _visibilityPolygon = polygon;
    }

    private void AddRandomPolygons(int count = 500)
    {
        obstacles.Clear();

        for (int i = 0; i < count; i++)
        {
            var polygon = RandomPolygonGenerator.GenerateRandomPolygon(CanvasWidth, CanvasHeight);

            obstacles.Add(polygon);
            canvas.Children.Add(polygon);
        }
    }

    private void CalculateSpatialIndex()
    {
        foreach (var obstacle in obstacles)
        {
            _spatialIndex?.Insert(obstacle);
        }
    }

    private void CalculateFrustum()
    {
        RemoveViews();

        var frustum = new FrustumCuller.ViewFrustum(
            _center,
            _frustumFOVAngle,
            _frustumDirection,
            _visibilityRange
        );

        (var visblePolygons, var fovPolygon) = _spatialIndex.QueryFOV(
            _center,
            _frustumFOVAngle,
            _frustumDirection,
            _visibilityRange
        );

        var filteredPolygons = FrustumCuller.GetVisiblePolygons(visblePolygons, frustum);

        _frustumPolygons.Clear();

        foreach (var visiblePolygon in filteredPolygons)
        {
            _frustumPolygons.Add((visiblePolygon, visiblePolygon.Fill));

            visiblePolygon.Fill = Brushes.Green;
        }

        fovPolygon.Fill = new SolidColorBrush(Color.FromArgb(128, 255, 255, 0));
        fovPolygon.Stroke = new SolidColorBrush(Colors.Yellow);
        fovPolygon.StrokeThickness = 2;

        _frustumPolygon = fovPolygon;

        canvas.Children.Add(_frustumPolygon);
    }

    private void RemoveViews()
    {
        if (_visibilityPolygon is not null)
        {
            canvas.Children.Remove(_visibilityPolygon);
        }

        if (_frustumPolygon is not null)
        {
            canvas.Children.Remove(_frustumPolygon);
        }

        foreach (var visiblePolygon in _frustumPolygons)
        {
            visiblePolygon.Item1.Fill = visiblePolygon.Item2;
        }
    }

    private void RadioButton_Click(object sender, RoutedEventArgs e)
    {
        ExecuteAlgorithm();
    }

    private void ExecuteAlgorithm()
    {
        if (fcRB.IsChecked == true)
        {
            TimingHelper.Time(() =>
            {
                CalculateFrustum();

                Console.WriteLine($"Polygon point count {_visibilityPolygon?.Points.Count}");
            }, nameof(CalculateFrustum));
        }
        else
        {
            TimingHelper.Time(() =>
            {
                CalculateVisibilty();

                Console.WriteLine($"Polygon point count {_visibilityPolygon?.Points.Count}");
            }, nameof(CalculateVisibilty));
        }
    }
}