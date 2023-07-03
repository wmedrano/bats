#!/usr/bin/guile -s
!#
(use-modules (srfi srfi-1))

;; TODO: Move the below to a module.
;; TODO: Avoid hardcoding target/debug directory.
(load-extension "target/debug/libbats" "init_bats")

;; TODO: Fold this functionality into make-track using keyword
;; arguments.
(define (make-track-with-plugin-id plugin-id)
  (let ((track-id (make-track)))
    (instantiate-plugin track-id plugin-id)
    track-id))

(define (delete-all-tracks)
  (count identity
         (map (lambda (t) (delete-track (assoc-ref t 'id)))
              (tracks))))

;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;; Examples
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
(define (make-track-for-all-instruments)
  (let* ((instrument-p       (lambda (plugin)
                               (assoc-ref plugin 'instrument?)))
         (plugin-to-id       (lambda (plugin)
                               (assoc-ref plugin 'id)))
         (instrument-plugins (filter instrument-p (plugins)))
         (plugin-ids         (map plugin-to-id instrument-plugins)))
    (map make-track-with-plugin-id
         plugin-ids)))
